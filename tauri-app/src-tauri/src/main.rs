// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eximd::config::FileSystem;
use eximd::exif::{ExifFile, FileNameGroup, FileNameGroupKey};
use eximd::file::FilePath;
use serde::ser::SerializeStruct;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use tauri::{AppHandle, Manager, Window};

// 1. load the source -> stick the dir in the state
// 2. collect files -> collect and group files into file groups -> emit a new file

#[derive(Default, Debug)]
struct AppState {
    source: Mutex<PathBuf>,
    file_group: Arc<Mutex<Vec<FileNameGroup>>>,
    exiffing_handles: Arc<Mutex<Vec<(JoinHandle<()>, Arc<AtomicBool>)>>>,
}

#[derive(Debug, Clone)]
struct FileNameGroupV(FileNameGroup);

fn exif_file_to_json(file: &ExifFile) -> serde_json::Value {
    serde_json::json!({
        "src": file.src.value(),
        "src_relative": file.src_relative.value(),
        "stem": file.stem.value(),
        "ext": file.ext.value(),
    })
}

impl serde::Serialize for FileNameGroupV {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("FileNameGroupV", 3)?;

        match &self.0 {
            FileNameGroup::Image { key, image, config } => {
                state.serialize_field("type", "Image")?;
                state.serialize_field("key", key.value())?;
                state.serialize_field("image", &exif_file_to_json(&image))?;
                state.serialize_field(
                    "config",
                    &config.iter().map(exif_file_to_json).collect::<Vec<_>>(),
                )?;
            }
            FileNameGroup::LiveImage {
                key,
                image,
                video,
                config,
            } => {
                state.serialize_field("type", "LiveImage")?;
                state.serialize_field("key", key.value())?;
                state.serialize_field("image", &exif_file_to_json(&image))?;
                state.serialize_field("video", &exif_file_to_json(&video))?;
                state.serialize_field(
                    "config",
                    &config.iter().map(exif_file_to_json).collect::<Vec<_>>(),
                )?;
            }
            FileNameGroup::Video { key, video, config } => {
                state.serialize_field("type", "Video")?;
                state.serialize_field("key", key.value())?;
                state.serialize_field("video", &exif_file_to_json(&video))?;
                state.serialize_field(
                    "config",
                    &config.iter().map(exif_file_to_json).collect::<Vec<_>>(),
                )?;
            }
            FileNameGroup::Uncertain {
                key,
                primary,
                config,
            } => {
                state.serialize_field("type", "Uncertain")?;
                state.serialize_field("key", key.value())?;
                state.serialize_field(
                    "primary",
                    &primary.iter().map(exif_file_to_json).collect::<Vec<_>>(),
                )?;
                state.serialize_field(
                    "config",
                    &config.iter().map(exif_file_to_json).collect::<Vec<_>>(),
                )?;
            }
            FileNameGroup::Unsupported { key, config } => {
                state.serialize_field("type", "Unsupported")?;
                state.serialize_field("key", key.value())?;
                state.serialize_field(
                    "config",
                    &config.iter().map(exif_file_to_json).collect::<Vec<_>>(),
                )?;
            }
        }

        state.end()
    }
}

#[derive(serde::Serialize, Clone)]
struct DropView {
    files: Vec<FileNameGroupV>,
    file_count: usize,
}

#[derive(Debug, serde::Deserialize)]
struct DropInputPayload {
    items: String,
}

#[derive(Debug, serde::Serialize, Clone)]
struct ExifFileData {
    key: String,
    src: PathBuf,
    src_next: PathBuf,
    file_name_next: String,
    ext: String,
}

impl ExifFileData {
    fn new(item: &ExifFile, next_src: &std::path::PathBuf) -> Self {
        Self {
            key: item.group_key.to_owned(),
            src: item.src.value().to_owned(),
            src_next: next_src.clone(),
            file_name_next: item
                .next_file_stem_from_exif()
                .unwrap_or("ERROR".to_string()),
            ext: item.ext.value().into(),
        }
    }
}

#[tauri::command]
fn cancel_exif_collection_cmd(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    let mut handles = state.exiffing_handles.lock().unwrap();

    for (_, flag) in handles.iter() {
        flag.store(true, Ordering::Relaxed);
    }

    while let Some((handle, _)) = handles.pop() {
        handle.join().expect("Could not join one of exif threads");
    }
    println!("All threads have been canceled and joined.");

    Ok(())
}

#[tauri::command]
fn start_exif_collection_cmd(
    app_handle: AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    window: Window,
) -> Result<(), String> {
    let mut file_group = { state.file_group.lock().unwrap().clone() };
    let resource_path = app_handle
        .path_resolver()
        .resolve_resource("../binaries")
        .ok_or_else(|| "Failed to resolve resource dir for exiftool")?;
    let state_clone = std::sync::Arc::clone(&state);
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let cancle_flag_clone = Arc::clone(&cancel_flag);

    let handle = thread::spawn(move || {
        let cmd_path = resource_path
            .join("exiftool/exiftool")
            .to_string_lossy()
            .to_string();

        for (i, mut group) in file_group.iter_mut().enumerate() {
            if cancle_flag_clone.load(Ordering::Relaxed) {
                println!("Exif collection thread cancelling");
                break;
            }

            match &mut group {
                FileNameGroup::Image { image, .. } => {
                    image.fetch_and_set_metadata(&cmd_path); // this is blocking.....
                    if let Some(next_src) = image.next_file_src_from_exif() {
                        let next_metadata = &image.metadata;
                        let mut file_group = state_clone.file_group.lock().unwrap();
                        if let FileNameGroup::Image { ref mut image, .. } = file_group[i] {
                            image.metadata = next_metadata.clone();
                        }
                        window
                            .emit("EXIF_FILE_DATA", ExifFileData::new(&image, &next_src))
                            .expect("send message to the FE");
                    } else {
                    }
                }
                FileNameGroup::Video { video, .. } => {
                    video.fetch_and_set_metadata(&cmd_path);
                    if let Some(next_src) = video.next_file_src_from_exif() {
                        let next_metadata = &video.metadata;
                        let mut file_group = state_clone.file_group.lock().unwrap();
                        if let FileNameGroup::Video { ref mut video, .. } = file_group[i] {
                            video.metadata = next_metadata.clone();
                        }
                        window
                            .emit("EXIF_FILE_DATA", ExifFileData::new(&video, &next_src))
                            .expect("send message to the FE");
                    }
                }
                FileNameGroup::LiveImage { image, .. } => {
                    image.fetch_and_set_metadata(&cmd_path);
                    if let Some(next_src) = image.next_file_src_from_exif() {
                        let next_metadata = &image.metadata;
                        let mut file_group = state_clone.file_group.lock().unwrap();
                        if let FileNameGroup::LiveImage { ref mut image, .. } = file_group[i] {
                            image.metadata = next_metadata.clone();
                        }
                        window
                            .emit("EXIF_FILE_DATA", ExifFileData::new(&image, &next_src))
                            .expect("send message to the FE");
                    }
                }
                _ => {
                    // Maybe create a new event that would notify the FE
                    // with the list of all the items we want to ignore?
                }
            }
        }

        window
            .emit("EXIF_COLLECTION_DONE", "")
            .expect("send message to FE");
    });

    state
        .exiffing_handles
        .lock()
        .unwrap()
        .push((handle, cancel_flag));

    Ok(())
}

struct TauriCommitNotifier<'a> {
    window: &'a Window,
    group_key: &'a FileNameGroupKey,
}

impl<'a> TauriCommitNotifier<'a> {
    fn new(window: &'a Window, group_key: &'a FileNameGroupKey) -> Self {
        Self { window, group_key }
    }
}

impl eximd::exif::ExifNotifier for TauriCommitNotifier<'_> {
    fn rename_success(&self, _prev: &FilePath, _next: &Path) -> () {
        self.window
            .emit("RENAME_COMMIT_SUCCESS_MSG", self.group_key)
            .expect("send message to FE");
    }

    fn rename_error(&self, _prev: &FilePath, err: String) -> () {
        eprintln!("{} -> {}", self.group_key.to_string(), err);
        unimplemented!();
    }

    fn rollback_success(&self, _next: &Path, _prev: &FilePath) -> () {
        println!("{} -> (ROLLBACK)", self.group_key.to_string(),);
        unimplemented!();
    }

    fn rollback_error(&self, _next: &Path, err: String) -> () {
        eprintln!(
            "ERROR: rolling back the {}: {}",
            self.group_key.to_string(),
            err
        );
        unimplemented!();
    }

    fn uncertain(&self, _src: &FilePath) -> () {
        println!("{} -> Uncertain Primary file", self.group_key.to_string());
        unimplemented!();
    }

    fn unsupported(&self, _src: &FilePath) -> () {
        println!("{} -> Unsupported file", self.group_key.to_string());
        unimplemented!();
    }
}

#[derive(Debug, serde::Deserialize)]
struct CommitRenamePayload {
    items: Vec<FileNameGroupKey>,
}

pub struct TempFileSystem {}

impl TempFileSystem {
    pub fn new() -> Self {
        Self {}
    }
}

impl FileSystem for TempFileSystem {
    fn rename(&self, prev: &Path, _next: &Path) -> std::io::Result<()> {
        println!("renaming {:?}", prev);
        Ok(())
    }
}

#[derive(Debug, serde::Serialize, Clone)]
struct RenameCounts {
    group_count: usize,
    file_count: usize,
}

#[tauri::command]
async fn commit_rename_groups_cmd(
    state: tauri::State<'_, Arc<AppState>>,
    window: Window,
    payload: CommitRenamePayload,
) -> Result<(), String> {
    // let fs = eximd::config::RealFileSystem::new(&eximd::config::RunType::Exec);
    let fs = TempFileSystem::new();
    let items = payload.items;
    let groups = {
        let file_groups = state.file_group.lock().unwrap();

        file_groups
            .iter()
            .filter(|x| items.iter().any(|y| y == x.group_key()))
            .map(|x| x.clone())
            .collect::<Vec<_>>()
    };

    thread::spawn(move || {
        let mut rename_group_count = 0;
        let mut rename_file_count = 0;
        thread::sleep(std::time::Duration::from_secs(1));

        for group in groups {
            let nf = TauriCommitNotifier::new(&window, group.group_key());
            match group {
                FileNameGroup::Image { ref image, .. } => {
                    if let Some(next_stem) = image.next_file_stem_from_exif() {
                        let file_count = eximd::exif::rename_with_rollback(
                            &fs,
                            &nf,
                            group.merge_into_rename_refs(),
                            &next_stem,
                        );
                        rename_group_count += 1;
                        rename_file_count += file_count;
                    }
                }
                FileNameGroup::Video { ref video, .. } => {
                    if let Some(next_stem) = video.next_file_stem_from_exif() {
                        let file_count = eximd::exif::rename_with_rollback(
                            &fs,
                            &nf,
                            group.merge_into_rename_refs(),
                            &next_stem,
                        );
                        rename_group_count += 1;
                        rename_file_count += file_count;
                    }
                }
                FileNameGroup::LiveImage { ref image, .. } => {
                    if let Some(next_stem) = image.next_file_stem_from_exif() {
                        let file_count = eximd::exif::rename_with_rollback(
                            &fs,
                            &nf,
                            group.merge_into_rename_refs(),
                            &next_stem,
                        );
                        rename_group_count += 1;
                        rename_file_count += file_count;
                    }
                }
                _ => eprintln!("Error: we are trying to rename a file we are not supposed to."),
            }
        }

        window
            .emit(
                "RENAME_COMMIT_DONE_MSG",
                RenameCounts {
                    group_count: rename_group_count,
                    file_count: rename_file_count,
                },
            )
            .expect("send message to FE");
    });

    Ok(())
}

#[tauri::command]
async fn drop_input_cmd(
    state: tauri::State<'_, Arc<AppState>>,
    payload: DropInputPayload,
) -> Result<PathBuf, String> {
    let input_path = std::path::Path::new(&payload.items);

    if input_path.exists() {
        let mut source = state.source.lock().unwrap();
        *source = input_path.to_path_buf().into();
        Ok(input_path.to_path_buf())
    } else {
        Err("Provided path does not seem to exist".to_string())
    }
}

#[tauri::command]
fn collect_rename_files_cmd(
    state: tauri::State<'_, Arc<AppState>>,
    window: Window,
) -> Result<(), String> {
    let input_path = { state.source.lock().unwrap().clone() };
    let state = std::sync::Arc::clone(&state);

    thread::spawn(move || match eximd::dir::collect_files(&input_path) {
        Ok(files) => {
            let file_count = files.len();
            let file_groups = eximd::exif::group_same_name_files(&files);

            let mut group = state.file_group.lock().unwrap();
            *group = file_groups;

            let files = group
                .iter()
                .map(|x| FileNameGroupV(x.clone()))
                .collect::<Vec<_>>();

            let res = DropView { files, file_count };
            window
                .emit("COLLECTION_SUCCESS", res)
                .expect("send message to FE to work");
        }
        Err(err) => {
            eprintln!("ERROR: we could not collect files {:?}", err);
        }
    });

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .manage(Arc::new(AppState::default()))
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // TODO: command cancle exif collection if the app canceles or drops new files
            drop_input_cmd,
            collect_rename_files_cmd,
            start_exif_collection_cmd,
            cancel_exif_collection_cmd,
            commit_rename_groups_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
