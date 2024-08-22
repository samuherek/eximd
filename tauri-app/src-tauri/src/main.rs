// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eximd::exif::{obj_str_from_array_of_one, ExifMetadata};
use eximd::file::{FileType, InputFile};
use eximd::utils;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::api::process::Command;
use tauri::{AppHandle, Manager, Runtime, Window};

// 1. load the source -> stick the dir in the state
// 2. collect files -> collect and group files into file groups -> emit a new file

#[derive(Default, Debug)]
struct FileGroupState {
    image: Option<InputFile>,
    configs: Vec<InputFile>,
    video: Option<InputFile>,
}

#[derive(Default, Debug)]
struct AppState {
    source: Mutex<PathBuf>,
    file_group: Arc<Mutex<HashMap<String, FileGroupState>>>,
}

pub fn get_exif_metadata(path: String) -> Option<ExifMetadata> {
    let cmd = Command::new(path)
        .args(["-j", "/Users/sam/Downloads/IMG_2483.jpg"])
        .output()
        .expect("to run exiftool command");

    let data = cmd.stdout;
    let value = match obj_str_from_array_of_one(&data) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("Error: {}", err);
            return None;
        }
    };
    match serde_json::from_str::<ExifMetadata>(&value) {
        Ok(value) => Some(value),
        Err(err) => {
            eprintln!("Error: {}", err);
            None
        }
    }
}

#[derive(Debug, serde::Serialize, Clone)]
struct FileRelated {
    path: PathBuf,
    relative_path: String,
    name: String,
    ext: String,
}

impl FileRelated {
    fn new(input: InputFile, input_path: &Path) -> Self {
        let relative_path = input
            .src
            .strip_prefix(input_path)
            .map(|x| format!("./{}", eximd::utils::path_to_string(x)))
            .expect("To have correct relative path");
        Self {
            path: input.src,
            relative_path,
            name: input.stem,
            ext: input.ext,
        }
    }
}

#[derive(Debug, serde::Serialize, Clone)]
struct FileView {
    path: PathBuf,
    relative_path: String,
    name: String,
    ext: String,
    file_type: FileType,
    file_configs: Vec<FileRelated>,
    file_live_photo: Option<FileRelated>,
}

impl FileView {
    fn new(input: InputFile, input_path: &Path) -> Self {
        let relative_path = input
            .src
            .strip_prefix(input_path)
            .map(|x| format!("./{}", eximd::utils::path_to_string(x)))
            .expect("To have correct relative path");
        Self {
            path: input.src,
            relative_path,
            name: input.stem,
            ext: input.ext,
            file_type: input.file_type,
            file_configs: vec![],
            file_live_photo: None,
        }
    }

    fn try_new(group: FileGroupState, input_path: &Path) -> Result<Self, String> {
        let mut main_file = group.image.map(|x| FileView::new(x, input_path));
        if let Some(video) = group.video {
            match main_file {
                Some(ref mut file) => {
                    file.file_live_photo = Some(FileRelated::new(video, input_path));
                }
                None => {
                    main_file = Some(FileView::new(video, input_path));
                }
            }
        }

        if let Some(ref mut main_file) = main_file {
            for config in group.configs {
                main_file
                    .file_configs
                    .push(FileRelated::new(config, input_path));
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

fn collect_files(input_path: PathBuf, window: Window) {
    thread::spawn(move || {
        let files = eximd::file::collect_files(&input_path);
        let mut file_map: HashMap<String, FileGroupState> = HashMap::new();

        for file in files {
            let entry = file_map
                .entry(file.hash_key())
                .or_insert(FileGroupState::default());

            match file.file_type {
                FileType::IMG => entry.image = Some(file),
                FileType::VIDEO => entry.video = Some(file),
                _ => entry.configs.push(file),
            }
        }

        let files = file_map
            .into_iter()
            .flat_map(|(_, group)| FileView::try_new(group, input_path.as_path()))
            .collect::<Vec<_>>();
        let file_count = files.len();

        let res = DropView { files, file_count };
        let _ = window.emit("FILES_COLLECTED", res).map_err(|err| {
            eprintln!("file collection error: {}", err);
        });
    });
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

    // let window_clone = window.clone();
    // let input_path_clone = input_path.to_path_buf();
    // collect_files(input_path_clone, window_clone);

    // let resource_path = app_handle
    //     .path_resolver()
    //     .resolve_resource("../binaries")
    //     .expect("failed to resolve resource dir");
    //
    // let _data = get_exif_metadata(
    //     resource_path
    //         .join("exiftool/exiftool")
    //         .to_string_lossy()
    //         .to_string(),
    // );
    // println!("data: {:?}", data);

    Ok(input_path.to_path_buf())
}

#[tauri::command]
fn collect_rename_files(
    state: tauri::State<'_, AppState>,
    window: Window,
    payload: DropInputPayload,
) -> Result<String, String> {
    println!("we have the sate {:?}", state);
    Ok("we are done".into())
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
        .invoke_handler(tauri::generate_handler![drop_input, collect_rename_files])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
