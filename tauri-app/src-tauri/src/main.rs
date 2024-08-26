// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eximd::exif::{ExifFile, FileNameGroup};
use serde::ser::SerializeStruct;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Manager, Window};

// 1. load the source -> stick the dir in the state
// 2. collect files -> collect and group files into file groups -> emit a new file

#[derive(Default, Debug)]
struct AppState {
    source: Mutex<PathBuf>,
    file_group: Arc<Mutex<Vec<FileNameGroup>>>,
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
                state.serialize_field("key", key)?;
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
                state.serialize_field("key", key)?;
                state.serialize_field("image", &exif_file_to_json(&image))?;
                state.serialize_field("video", &exif_file_to_json(&video))?;
                state.serialize_field(
                    "config",
                    &config.iter().map(exif_file_to_json).collect::<Vec<_>>(),
                )?;
            }
            FileNameGroup::Video { key, video, config } => {
                state.serialize_field("type", "Video")?;
                state.serialize_field("key", key)?;
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
                state.serialize_field("key", key)?;
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
                state.serialize_field("key", key)?;
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
    items: Vec<String>,
}

#[derive(Debug, serde::Serialize, Clone)]
struct ExifFileData {
    key: String,
    src: PathBuf,
    src_next: PathBuf,
    file_name_next: String,
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
        }
    }
}

#[tauri::command]
fn start_exif_collection(
    app_handle: AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    window: Window,
) -> Result<(), String> {
    let mut file_group = { state.file_group.lock().unwrap().clone() };
    let resource_path = app_handle
        .path_resolver()
        .resolve_resource("../binaries")
        .ok_or_else(|| "Failed to resolve resource dir for exiftool")?;
    let state = std::sync::Arc::clone(&state);

    thread::spawn(move || {
        let cmd_path = resource_path
            .join("exiftool/exiftool")
            .to_string_lossy()
            .to_string();
        for (i, mut group) in file_group.iter_mut().enumerate() {
            match &mut group {
                FileNameGroup::Image { image, .. } => {
                    image.fetch_and_set_metadata(&cmd_path); // this is blocking.....
                    if let Some(next_src) = image.next_file_src_from_exif() {
                        let next_metadata = &image.metadata;
                        let mut file_group = state.file_group.lock().unwrap();
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
                        let mut file_group = state.file_group.lock().unwrap();
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
                        let mut file_group = state.file_group.lock().unwrap();
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

    Ok(())
}

#[tauri::command]
async fn drop_input(
    state: tauri::State<'_, Arc<AppState>>,
    payload: DropInputPayload,
) -> Result<PathBuf, String> {
    if payload.items.len() != 1 {
        return Err("You need to provide one path.".into());
    }

    let input_path = payload
        .items
        .get(0)
        .map(std::path::Path::new)
        .ok_or_else(|| "Invalid path to the resources")?;

    let mut source = state.source.lock().unwrap();
    *source = input_path.to_path_buf().into();

    Ok(input_path.to_path_buf())
}

#[tauri::command]
fn collect_rename_files(state: tauri::State<'_, Arc<AppState>>) -> Result<DropView, String> {
    let input_path = state.source.lock().unwrap();
    let files = eximd::dir::collect_files(&input_path)?;
    let file_count = files.len();
    let file_groups = eximd::exif::group_same_name_files(&files);

    let mut group = state.file_group.lock().unwrap();
    *group = file_groups;

    let files = group
        .iter()
        .map(|x| FileNameGroupV(x.clone()))
        .collect::<Vec<_>>();

    let res = DropView { files, file_count };
    Ok(res)
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
            drop_input,
            collect_rename_files,
            start_exif_collection 
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
