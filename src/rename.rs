use super::exif;
use super::utils;
use serde::Deserialize;
use std::error::Error;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct DateTime {
    source_file: String,
    file_name: String,
    #[serde(default, deserialize_with = "parse_date")]
    date_time_original: Option<chrono::NaiveDateTime>,
}

fn parse_date<'de, D>(deserializer: D) -> Result<Option<chrono::NaiveDateTime>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    if let Some(s) = s {
        match chrono::NaiveDateTime::parse_from_str(&s, "%Y:%m:%d %H:%M:%S") {
            Ok(dt) => Ok(Some(dt)),
            Err(_) => Ok(None),
        }
    } else {
        Ok(None)
    }
}

pub struct OldPath(String);

impl OldPath {
    pub fn new(value: &str) -> Self {
        OldPath(value.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub struct NewPath(String);

impl NewPath {
    pub fn new(value: &str) -> Self {
        NewPath(value.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn get_new_path<P: AsRef<Path>>(path: P) -> Result<(OldPath, NewPath), Box<dyn Error>> {
    let path = path.as_ref();
    let path_str = path.to_str().expect("To convert the path to string");
    let old_path = OldPath::new(path_str);
    let cmd = Command::new("exiftool")
        .args(["-j", path.to_str().unwrap()])
        .output()
        .expect("Exiftool command did not work");

    let data = String::from_utf8(cmd.stdout).expect("To convert the utf8 into a string");
    let value = exif::get_one_exif_input(&data)?;
    let value = serde_json::from_str::<DateTime>(&value).map_err(|err| {
        println!("Error in path: {}", path_str);
        err
    })?;
    let ext = value.file_name.split(".").last().unwrap_or("");
    if let Some(date) = value.date_time_original {
        let date_name = date.format("%Y-%m-%d_%H.%M.%S");
        let name = format!("{}.{}", date_name, ext);
        let new_path = path.with_file_name(name);
        let new_path = new_path
            .as_path()
            .to_str()
            .expect("To convert new path to string");
        let new_path = NewPath::new(new_path);
        Ok((old_path, new_path))
    } else {
        Err(format!(
            "Could not get the exif data from {:?}",
            path.to_str()
        ))
        .map_err(|e| e.into())
    }
}

pub fn get_new_paths<P: AsRef<Path>>(path: P) -> Result<Vec<(OldPath, NewPath)>, Box<dyn Error>> {
    let path = path.as_ref();
    let mut items = vec![];
    if path.is_dir() {
        for entry in WalkDir::new(path) {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_file() && (utils::is_img_ext(path) || utils::is_video_ext(path)) {
                    // println!("entry: {:?}", entry);
                    match get_new_path(path) {
                        Ok(val) => items.push(val),
                        Err(_) => {
                            println!(
                                "Skipping: missing exif date: {}",
                                path.to_str().unwrap_or("")
                            )
                        }
                    }
                }
            }
        }
    } else if path.is_file() {
        // println!("we are in the file {:?}", path);
        match get_new_path(path) {
            Ok(val) => items.push(val),
            Err(_) => {
                println!(
                    "Skipping: missing exif date: {}",
                    path.to_str().unwrap_or("")
                )
            }
        }
    } else {
        eprintln!("We don't support antying but a directory or a file.");
    }

    Ok(items)
}
