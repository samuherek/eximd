// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eximd::exif::FileNameGroup;
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

#[derive(Debug, serde::Serialize, Clone)]
struct FileNameGroupView {
    src: String,
    src_relative: String,
    stem: String,
    ext: String,
    file_type: FileType,
    file_configs: Vec<FileRelated>,
    file_live_photo: Option<FileRelated>,
    error: Option<String>,
}

fn file_views_from_file_name_group(group: &FileNameGroup) -> Vec<FileNameGroupView> {
    match group {
        FileNameGroup::Image { image, config } => {
            vec![FileNameGroupView {
                src: image.src.to_string(),
                src_relative: image.src_relative.to_string(),
                stem: image.stem.to_string(),
                ext: image.ext.to_string(),
                file_type: image.file_type.clone(),
                file_configs: config
                    .iter()
                    .map(|x| FileRelated {
                        src: x.src.to_string(),
                        src_relative: x.src_relative.to_string(),
                        stem: x.stem.to_string(),
                        ext: x.ext.to_string(),
                    })
                    .collect(),
                file_live_photo: None,
                error: None,
            }]
        }
        FileNameGroup::LiveImage {
            image,
            video,
            config,
        } => {
            vec![FileNameGroupView {
                src: image.src.to_string(),
                src_relative: image.src_relative.to_string(),
                stem: image.stem.to_string(),
                ext: image.ext.to_string(),
                file_type: image.file_type.clone(),
                file_configs: config
                    .iter()
                    .map(|x| FileRelated {
                        src: x.src.to_string(),
                        src_relative: x.src_relative.to_string(),
                        stem: x.stem.to_string(),
                        ext: x.ext.to_string(),
                    })
                    .collect(),
                file_live_photo: Some(FileRelated {
                    src: video.src.to_string(),
                    src_relative: video.src_relative.to_string(),
                    stem: video.stem.to_string(),
                    ext: video.ext.to_string(),
                }),
                error: None,
            }]
        }
        FileNameGroup::Video { video, config } => {
            vec![FileNameGroupView {
                src: video.src.to_string(),
                src_relative: video.src_relative.to_string(),
                stem: video.stem.to_string(),
                ext: video.ext.to_string(),
                file_type: video.file_type.clone(),
                file_configs: config
                    .iter()
                    .map(|x| FileRelated {
                        src: x.src.to_string(),
                        src_relative: x.src_relative.to_string(),
                        stem: x.stem.to_string(),
                        ext: x.ext.to_string(),
                    })
                    .collect(),
                file_live_photo: None,
                error: None,
            }]
        }
        FileNameGroup::Uncertain(config) => config
            .iter()
            .map(|x| FileNameGroupView {
                src: x.src.to_string(),
                src_relative: x.src_relative.to_string(),
                stem: x.stem.to_string(),
                ext: x.ext.to_string(),
                file_type: x.file_type.clone(),
                file_configs: vec![],
                file_live_photo: None,
                error: None,
            })
            .collect(),
    }
}

#[derive(serde::Serialize, Clone)]
struct DropView {
    files: Vec<FileNameGroupView>,
    file_count: usize,
}

#[derive(Debug, serde::Deserialize)]
struct DropInputPayload {
    items: Vec<String>,
}

#[derive(Debug, serde::Serialize, Clone)]
struct ExifFileData {
    idx: usize,
    src: PathBuf,
    src_next: PathBuf,
    stem_next: String,
}

#[derive(Debug, serde::Serialize, Clone)]
struct ExifFileUncertain {
    src: PathBuf,
}

#[tauri::command]
fn start_exif_collection(
    app_handle: AppHandle,
    state: tauri::State<'_, AppState>,
    window: Window,
) -> Result<(), String> {
    let mut file_group = state.file_group.lock().unwrap().clone();
    let resource_path = app_handle
        .path_resolver()
        .resolve_resource("../binaries")
        .ok_or_else(|| "Failed to resolve resource dir for exiftool")?;

    thread::spawn(move || {
        let cmd_path = resource_path
            .join("exiftool/exiftool")
            .to_string_lossy()
            .to_string();
        for (i, mut group) in file_group.iter_mut().enumerate() {
            match &mut group {
                FileNameGroup::Image { image, .. } => {
                    image.fetch_and_set_metadata(&cmd_path);
                    if let Some(next_src) = image.next_file_src_from_exif() {
                        // let mut file_group = state.file_group.lock().unwrap();
                        // *file_group[i].image = image.clone();
                        window
                            .emit(
                                "EXIF_FILE_DATA",
                                ExifFileData {
                                    idx: i,
                                    src: image.src.value().to_owned(),
                                    src_next: next_src.clone(),
                                    stem_next: image
                                        .next_file_stem_from_exif()
                                        .unwrap_or("ERROR".to_string()),
                                },
                            )
                            .expect("send message to the FE");
                    } else {
                    }
                }
                FileNameGroup::Video { video, .. } => {
                    video.fetch_and_set_metadata(&cmd_path);
                    if let Some(next_src) = video.next_file_src_from_exif() {
                        // rename_with_rollback(nf, group.merge_into_refs(), &next_src);
                        window
                            .emit(
                                "EXIF_FILE_DATA",
                                ExifFileData {
                                    idx: i,
                                    src: video.src.value().to_owned(),
                                    src_next: next_src.clone(),
                                    stem_next: video
                                        .next_file_stem_from_exif()
                                        .unwrap_or("ERROR".to_string()),
                                },
                            )
                            .expect("send message to the FE");
                    }
                }
                FileNameGroup::LiveImage { image, .. } => {
                    image.fetch_and_set_metadata(&cmd_path);
                    if let Some(next_src) = image.next_file_src_from_exif() {
                        // rename_with_rollback(nf, group.merge_into_refs(), &next_src);
                        window
                            .emit(
                                "EXIF_FILE_DATA",
                                ExifFileData {
                                    idx: i,
                                    src: image.src.value().to_owned(),
                                    src_next: next_src.clone(),
                                    stem_next: image
                                        .next_file_stem_from_exif()
                                        .unwrap_or("ERROR".to_string()),
                                },
                            )
                            .expect("send message to the FE");
                    }
                }
                FileNameGroup::Uncertain(_) => {}
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
        .flat_map(|x| file_views_from_file_name_group(x))
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
