// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eximd::exif::{get_exif_file_from_input, ExifFile, FileNameGroup};
use eximd::file::FileType;
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

#[derive(Debug, serde::Serialize, Clone)]
struct FileRelated {
    src: String,
    src_relative: String,
    stem: String,
    ext: String,
}

impl FileRelated {
    fn new(input: &ExifFile) -> Self {
        Self {
            src: input.src.to_string(),
            src_relative: input.src.to_string(),
            stem: input.stem.to_string(),
            ext: input.ext.to_string(),
        }
    }
}

#[derive(Debug, serde::Serialize, Clone)]
struct FileView {
    src: String,
    src_relative: String,
    stem: String,
    ext: String,
    file_type: FileType,
    file_configs: Vec<FileRelated>,
    file_live_photo: Option<FileRelated>,
    error: Option<String>,
}

impl FileView {
    fn new(input: &ExifFile) -> Self {
        Self {
            src: input.src.to_string(),
            src_relative: input.src_relative.to_string(),
            stem: input.stem.to_string(),
            ext: input.ext.to_string(),
            file_type: input.file_type.clone(),
            file_configs: vec![],
            file_live_photo: None,
        }
    }

    fn try_new(group: AppFileGroup) -> Result<Self, String> {
        // 1. if there is more primary files than one, then

        let mut main_file = group.primary.map(|x| FileView::new(&ExifFile::from(x)));

        if let Some(video) = group.video {
            match main_file {
                Some(ref mut file) => {
                    file.file_live_photo = Some(FileRelated::new(&ExifFile::from(video)));
                }
                None => {
                    main_file = Some(FileView::new(&ExifFile::from(video)));
                }
            }
        }

        if let Some(ref mut main_file) = main_file {
            for config in group.configs {
                main_file
                    .file_configs
                    .push(FileRelated::new(&ExifFile::from(config)));
            }
        }

        main_file.ok_or("Could not create a file group.".into())
    }
}

#[derive(serde::Serialize, Clone)]
struct DropView {
    files: Vec<FileView>,
    file_count: usize,
}

#[derive(Debug, serde::Deserialize)]
struct DropInputPayload {
    items: Vec<String>,
}

#[tauri::command]
fn start_exif_collection(
    app_handle: AppHandle,
    state: tauri::State<'_, AppState>,
    window: Window,
) -> Result<(), String> {
    let file_group = state.file_group.lock().unwrap().clone();
    let resource_path = app_handle
        .path_resolver()
        .resolve_resource("../binaries")
        .ok_or_else(|| "Failed to resolve resource dir for exiftool")?;

    thread::spawn(move || {
        let cmd_path = resource_path
            .join("exiftool/exiftool")
            .to_string_lossy()
            .to_string();
        for (i, file_group) in file_group.iter().enumerate() {
            if let Some(image) = &file_group.image {
                let data = get_exif_file_from_input(&cmd_path, image);
                println!("processsed {:?}", data);
                window
                    .emit("EXIF_FILE_DATA", "")
                    .expect("to emit event to the FE");
            }
        }
    });

    Ok(())
}

#[tauri::command]
async fn drop_input(
    state: tauri::State<'_, AppState>,
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
fn collect_rename_files(state: tauri::State<'_, AppState>) -> Result<DropView, String> {
    let input_path = state.source.lock().unwrap();
    let files = eximd::dir::collect_files(&input_path)?;
    let file_groups = eximd::exif::group_same_name_files(&files);

    let mut group_map = state.file_group.lock().unwrap();
    *group_map = file_groups;

    let files = group_map
        .iter()
        .flat_map(|group| FileView::try_new(group.clone()))
        .collect::<Vec<_>>();
    let file_count = files.len();

    let res = DropView { files, file_count };
    Ok(res)
}

fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            drop_input,
            collect_rename_files,
            start_exif_collection
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
