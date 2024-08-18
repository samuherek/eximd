// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eximd::exif;
use tauri::async_runtime::spawn;
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

#[tauri::command]
async fn drop_input(payload: DropInputPayload) -> Result<Vec<String>, String> {
    if payload.items.len() > 1 {
        return Err("We accept only one input file now.".into());
    }
    let files = payload
        .items
        .get(0)
        .map(|x| std::path::Path::new(x))
        .map(|x| eximd::file::collect_files(x));

    match files {
        Some(files) => Ok(files
            .iter()
            .map(|x| eximd::utils::path_to_string(x.path()))
            .collect::<Vec<_>>()),
        None => Err("We could not resolve the path.".into()),
    }
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
