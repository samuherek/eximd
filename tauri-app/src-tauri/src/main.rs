// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eximd::file::{FileType, InputFile};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[derive(Debug, serde::Deserialize)]
struct DropInputPayload {
    items: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
struct FileConfig {
    path: PathBuf,
    relative_path: String,
    name: String,
    ext: String,
}

impl FileConfig {
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

#[derive(Debug, serde::Serialize)]
struct FileView {
    path: PathBuf,
    relative_path: String,
    name: String,
    ext: String,
    file_type: FileType,
    file_configs: Vec<FileConfig>,
    file_live_photo: Option<FileConfig>,
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

    fn try_new(group: FileGroup, input_path: &Path) -> Result<Self, String> {
        let mut main_file = group.image.map(|x| FileView::new(x, input_path));
        if let Some(video) = group.video {
            match main_file {
                Some(ref mut file) => {
                    file.file_live_photo = Some(FileConfig::new(video, input_path));
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
                    .push(FileConfig::new(config, input_path));
            }
        }

        main_file.ok_or("Could not create a file group.".into())
    }
}

#[derive(Default)]
struct FileGroup {
    image: Option<InputFile>,
    configs: Vec<InputFile>,
    video: Option<InputFile>,
}

#[derive(serde::Serialize)]
struct DropView {
    directory: PathBuf,
    files: Vec<FileView>,
    file_count: usize,
}

#[tauri::command]
async fn drop_input(payload: DropInputPayload) -> Result<DropView, String> {
    if payload.items.len() != 1 {
        return Err("We accept only one input file now.".into());
    }
    let input_path = payload
        .items
        .get(0)
        .map(|x| std::path::Path::new(x))
        .expect("Should have a path");
    let files = eximd::file::collect_files(&input_path);
    let file_count = files.len();
    let mut file_map: HashMap<String, FileGroup> = HashMap::new();

    for file in files {
        let entry = file_map
            .entry(file.hash_key())
            .or_insert(FileGroup::default());

        match file.file_type {
            FileType::IMG => entry.image = Some(file),
            FileType::VIDEO => entry.video = Some(file),
            _ => entry.configs.push(file),
        }
    }

    let files = file_map
        .into_iter()
        .flat_map(|(_, group)| FileView::try_new(group, input_path))
        .collect::<Vec<_>>();

    let res = DropView {
        directory: input_path.to_path_buf(),
        files,
        file_count,
    };

    Ok(res)
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet, drop_input])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
