use super::utils;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FilePath(PathBuf);

impl FilePath {
    pub fn new(path: &Path) -> Self {
        Self(path.to_path_buf())
    }

    pub fn value(&self) -> &PathBuf {
        &self.0
    }

    pub fn as_str(&self) -> &str {
        &self.value().to_str().unwrap_or_default()
    }

    pub fn with_file_name<S: AsRef<OsStr>>(&self, file_name: S) -> PathBuf {
        self.value().to_owned().with_file_name(file_name)
    }
}

impl ToString for FilePath {
    fn to_string(&self) -> String {
        self.value().to_string_lossy().into()
    }
}

#[derive(Debug, Clone)]
pub struct FileStem(String);

impl FileStem {
    // TODO: maybe it needs to reutrn an option or an error in case
    // we can not convert it to string?
    pub fn new(path: &Path) -> Self {
        let stem = path
            .file_stem()
            .expect("To have a file stem")
            .to_string_lossy()
            .into();
        Self(stem)
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

impl ToString for FileStem {
    fn to_string(&self) -> String {
        self.value().to_owned().into()
    }
}

#[derive(Debug, Clone)]
pub struct FileExt(String);

impl FileExt {
    // TODO: maybe it needs to reutrn an option or an error in case
    // we can not convert it to string?
    pub fn new(path: &Path) -> Self {
        let ext = path
            .extension()
            .map(|i| i.to_string_lossy().into())
            .unwrap_or_default();
        Self(ext)
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

impl ToString for FileExt {
    fn to_string(&self) -> String {
        self.value().to_owned().into()
    }
}

#[derive(Debug, serde::Serialize, Clone)]
pub enum FileType {
    IMG,
    VIDEO,
    OTHER,
}

impl From<&FileExt> for FileType {
    fn from(ext: &FileExt) -> Self {
        if utils::is_img(ext.value()) {
            FileType::IMG
        } else if utils::is_video(ext.value()) {
            FileType::VIDEO
        } else {
            FileType::OTHER
        }
    }
}

#[derive(Debug, Clone)]
pub struct InputFile {
    pub src: FilePath,
    pub src_relative: FilePath,
    pub stem: FileStem,
    pub ext: FileExt,
    pub file_type: FileType,
}

impl InputFile {
    pub fn new(absolute_path: &FilePath, relative_point: &Path) -> Self {
        let src = absolute_path.clone();
        let relative_path = absolute_path.value().to_owned();
        let relative_path = relative_path
            .strip_prefix(relative_point)
            .expect("to strip the drop path prefix from foun file");
        let src_relative = FilePath::new(relative_path);
        let stem = FileStem::new(absolute_path.value());
        let ext = FileExt::new(absolute_path.value());
        let file_type = FileType::from(&ext);
        Self {
            src,
            src_relative,
            stem,
            ext,
            file_type,
        }
    }

    pub fn hash_key(&self) -> String {
        self.stem.to_string()
    }
}
