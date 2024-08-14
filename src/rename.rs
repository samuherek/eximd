use super::exif;
use super::utils;
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use walkdir::WalkDir;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct DateTime {
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

fn exif_date_time<P: AsRef<Path>>(path: P) -> Option<DateTime> {
    let cmd = Command::new("exiftool")
        .args(["-j", path.as_ref().to_str().unwrap()])
        .output()
        .expect("Exiftool command did not work");

    let data = String::from_utf8(cmd.stdout).expect("To convert the utf8 into a string");
    let value = match exif::get_one_exif_input(&data) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("Error: {}", err);
            return None;
        }
    };
    match serde_json::from_str::<DateTime>(&value) {
        Ok(value) => Some(value),
        Err(err) => {
            eprintln!("Error: {}", err);
            None
        }
    }
}

fn next_file_stem(date: &DateTime) -> Option<String> {
    if let Some(date) = date.date_time_original {
        return Some(date.format("%Y-%m-%d_%H.%M.%S").to_string());
    }
    None
}

#[derive(PartialEq)]
pub enum RunType {
    Dry,
    Exec,
}

fn rename_file(old_path: OldPath, new_path: NewPath) -> Result<(), Box<dyn Error>> {
    let old_path = std::path::Path::new(old_path.as_str());
    let new_path = std::path::Path::new(new_path.as_str());
    let _ = std::fs::rename(old_path, new_path)?;
    Ok(())
}

#[derive(Debug)]
pub struct InputSrc {
    source: PathBuf,
}

impl InputSrc {
    fn new(source: PathBuf) -> Self {
        Self { source }
    }

    fn source_string(&self) -> String {
        self.source.to_string_lossy().into()
    }

    fn key(&self) -> String {
        match self.source.file_stem() {
            Some(stem) => stem.to_string_lossy().into(),
            None => "".into(),
        }
    }
}

fn is_supported(path: &Path) -> bool {
    path.is_file() && (utils::is_img_ext(path) || utils::is_video_ext(path))
}

fn walk_path(path: &PathBuf) -> Result<Vec<InputSrc>, Box<dyn Error>> {
    let p = path.as_path();
    let mut paths = vec![];
    if p.is_file() {
        paths.push(InputSrc::new(p.to_path_buf()));
    } else if p.is_dir() {
        for entry in WalkDir::new(p) {
            if let Ok(entry) = entry {
                paths.push(InputSrc::new(entry.path().to_path_buf()));
            }
        }
    }

    Ok(paths)
}

struct PrepInput {
    input: String,
    output: String,
}

// impl PrepInput {
//     fn new() -> Self {}
// }

fn hash_map_input(input: Vec<InputSrc>) -> HashMap<String, Vec<InputSrc>> {
    let mut map = HashMap::new();

    for item in input {
        map.entry(item.key()).or_insert(Vec::new()).push(item);
    }

    map
}

pub fn complex_paths(path: PathBuf, mode: RunType) -> Result<(), Box<dyn Error>> {
    let input = walk_path(&path)?;
    let map = hash_map_input(input);

    for (key, items) in map {
        let supported_input = items
            .iter()
            .filter(|val| is_supported(&val.source))
            .collect::<Vec<_>>();
        if supported_input.len() > 1 {
            eprintln!(
                "We got more supported files with the same name for: {}",
                key
            );
            println!("Skipping...");
            continue;
        }

        if let Some(item) = supported_input.get(0) {
            let next_stem = exif_date_time(&item.source)
                .as_ref()
                .map(next_file_stem)
                .flatten();
            if let Some(stem) = next_stem {
                for input in items.iter() {
                    let mut name = stem.clone();
                    if let Some(ext) = input.source.extension() {
                        name.push_str(".");
                        let ext = ext.to_string_lossy().to_string();
                        name.push_str(ext.as_str());
                    }
                    let next_path = input.source.with_file_name(name);
                    println!(
                        "{} -> {}",
                        item.source_string(),
                        next_path.display().to_string()
                    );
                }
            }
        }
    }

    Ok(())
}

pub fn get_new_paths<P: AsRef<Path>>(path: P, mode: RunType) -> Result<(), Box<dyn Error>> {
    let path = path.as_ref();
    if path.is_dir() {
        for entry in WalkDir::new(path) {
            if let Ok(entry) = entry {
                let file_path = entry.path();
                let is_processable =
                    utils::is_img_ext(&file_path) || utils::is_video_ext(&file_path);
                if file_path.is_file() && is_processable {
                    match get_new_path(&file_path) {
                        Ok(val) => match mode {
                            RunType::Dry => {
                                println!("{} -> {}", val.0.as_str(), val.1.as_str());
                            }
                            RunType::Exec => {
                                println!("{} -> {}", val.0.as_str(), val.1.as_str());
                                rename_file(val.0, val.1)?;
                            }
                        },
                        Err(_) => {
                            let path_str = file_path.to_str().unwrap_or("XXX");
                            println!("Missing date: {}", path_str);
                        }
                    }
                } else {
                    let path_str = file_path.to_str().unwrap_or("XXX");
                    println!("Skipping: {}", path_str);
                }
            }
        }
    } else if path.is_file() {
        match get_new_path(path) {
            Ok(val) => match mode {
                RunType::Dry => {
                    println!("{} -> {}", val.0.as_str(), val.1.as_str());
                }
                RunType::Exec => {
                    println!("{} -> {}", val.0.as_str(), val.1.as_str());
                    rename_file(val.0, val.1)?;
                }
            },
            Err(_) => {
                let path_str = path.to_str().unwrap_or("XXX");
                println!("Missing date: {}", path_str);
            }
        }
    } else {
        eprintln!("We don't support antying but a directory or a file.");
    }

    Ok(())
}
