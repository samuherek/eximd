use super::utils;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

struct FilePath(PathBuf);

impl FilePath {
    fn new(path: &Path) -> Self {
        Self(path.to_path_buf())
    }

    fn value(&self) -> &PathBuf {
        &self.0
    }
}

#[derive(Debug, serde::Serialize, Clone)]
pub enum FileType {
    IMG,
    VIDEO,
    OTHER,
}

pub struct InputFile {
    pub src: PathBuf,
    pub stem: String,
    pub ext: String,
    pub file_type: FileType,
}

impl InputFile {
    fn new(file: &FilePath) -> Self {
        let src = file.value().to_owned();
        let stem = get_stem(&src);
        let ext = get_ext(&src);
        let file_type = file_type_from_str(&ext);
        Self {
            src,
            stem,
            ext,
            file_type,
        }
    }

    pub fn hash_key(&self) -> String {
        self.stem.clone()
    }

    pub fn path(&self) -> &PathBuf {
        &self.src
    }
}

fn get_stem(path: &Path) -> String {
    path.file_stem()
        .expect("To have a file stem")
        .to_string_lossy()
        .into()
}

fn get_ext(path: &Path) -> String {
    path.extension()
        .map(|i| i.to_string_lossy().into())
        .unwrap_or_default()
}

fn file_type_from_str(ext: &str) -> FileType {
    if utils::is_img(ext) {
        FileType::IMG
    } else if utils::is_video(ext) {
        FileType::VIDEO
    } else {
        FileType::OTHER
    }
}

// Accept either a directory or a file path.
// If it is a file, it will return a vector of just one file.
// If it is a directory, it will walk the files and return
// all the files recursivelly.
pub fn collect_files(path: &Path) -> Vec<InputFile> {
    let mut files = vec![];
    // We support direct path
    if path.is_file() {
        let file = FilePath::new(path);
        files.push(InputFile::new(&file));
    // We support a directory and we walk all the paths.
    } else if path.is_dir() {
        for entry in WalkDir::new(path) {
            let entry = entry.map_or(None, |x| {
                let x_path = x.path();
                if x_path.is_file() {
                    Some(FilePath::new(x_path))
                } else {
                    None
                }
            });
            if let Some(entry) = entry {
                files.push(InputFile::new(&entry));
            }
        }
    } else {
        eprintln!(
            "Error: path is neither a file niether a dir: {}",
            path.display().to_string()
        );
    }

    files
}
