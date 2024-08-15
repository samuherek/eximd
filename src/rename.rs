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

fn rename_file(old_path: &PathBuf, new_path: &PathBuf) -> Result<(), Box<dyn Error>> {
    // let old_path = std::path::Path::new(old_path.as_str());
    // let new_path = std::path::Path::new(new_path.as_str());
    let _ = std::fs::rename(old_path, new_path)?;
    Ok(())
}

#[derive(Debug)]
pub struct InputSrc(PathBuf);

impl InputSrc {
    fn new(path: &PathBuf) -> Option<Self> {
        let p = path.clone();
        if p.as_path().is_file() {
            Some(Self(p))
        } else {
            None
        }
    }

    fn source(&self) -> &PathBuf {
        &self.0
    }

    fn source_string(&self) -> String {
        self.source().to_string_lossy().into()
    }

    fn key(&self) -> String {
        match self.source().file_stem() {
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
        if let Some(input) = InputSrc::new(path) {
            paths.push(input);
        }
    } else if p.is_dir() {
        for entry in WalkDir::new(p) {
            let val = entry.map_or(None, |x| InputSrc::new(&x.path().to_path_buf()));
            if let Some(input) = val {
                paths.push(input);
            }
        }
    }

    Ok(paths)
}

struct PrepInput {
    source: PathBuf,
    next_stem: String,
    ext: String,
}

impl PrepInput {
    fn new(input: &InputSrc, next_stem: &str) -> Self {
        let ext = input
            .source()
            .extension()
            .map(|i| i.to_string_lossy().into())
            .unwrap_or("".into());

        Self {
            source: input.source().to_path_buf(),
            next_stem: next_stem.into(),
            ext,
        }
    }

    fn source(&self) -> PathBuf {
        self.source.clone()
    }

    fn source_string(&self) -> String {
        self.source.to_string_lossy().into()
    }

    fn next_file_name(&self) -> String {
        format!("{}.{}", self.next_stem, self.ext)
    }

    fn next_path(&self) -> PathBuf {
        self.source.with_file_name(self.next_file_name())
    }

    fn next_path_string(&self) -> String {
        self.next_path().to_string_lossy().into()
    }
}

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

    if mode == RunType::Dry {
        println!("Dry run::");
    }

    for (key, items) in map {
        let supported_input = items
            .iter()
            .filter(|val| is_supported(&val.source()))
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
            let next_stem = exif_date_time(&item.source())
                .as_ref()
                .map(next_file_stem)
                .flatten();
            if let Some(stem) = next_stem {
                let queue = items
                    .iter()
                    .map(|i| PrepInput::new(i, &stem))
                    .collect::<Vec<PrepInput>>();
                rename_file_group(&queue, &mode)?;
            }
        } else {
            for item in items.iter() {
                println!("{} -> Skip", item.source_string());
            }
        }
    }

    Ok(())
}

fn rename_file_group(group: &[PrepInput], mode: &RunType) -> Result<(), Box<dyn Error>> {
    // TODO: Rollback if there is an error in the renaming.
    // let mut done = vec![];
    for input in group {
        match mode {
            RunType::Exec => match rename_file(&input.source(), &input.next_path()) {
                Ok(_) => {
                    println!("{} -> {}", input.source_string(), input.next_path_string());
                }
                Err(err) => {
                    eprintln!("{} -> {}", input.source_string(), err);
                }
            },
            RunType::Dry => {
                println!("{} -> {}", input.source_string(), input.next_path_string());
            }
        }
    }

    Ok(())
}
