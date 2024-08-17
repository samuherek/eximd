use super::config::RunType;
use std::path::{Path, PathBuf};

pub trait FileSystem {
    fn rename(&self, prev: &Path, next: &Path) -> std::io::Result<()>;
}

pub struct RealFileSystem {
    mode: RunType,
}

impl RealFileSystem {
    pub fn new(mode: &RunType) -> Self {
        Self { mode: mode.clone() }
    }
}

impl FileSystem for RealFileSystem {
    fn rename(&self, prev: &Path, next: &Path) -> std::io::Result<()> {
        if self.mode == super::config::RunType::Exec {
            std::fs::rename(prev, next)?;
        }
        Ok(())
    }
}

pub struct MockFileSystem {
    pub renamed_files: std::cell::RefCell<Vec<(PathBuf, PathBuf)>>,
}

impl MockFileSystem {
    pub fn _new() -> Self {
        Self {
            renamed_files: std::cell::RefCell::new(vec![]),
        }
    }
}

impl FileSystem for MockFileSystem {
    fn rename(&self, prev: &Path, next: &Path) -> std::io::Result<()> {
        self.renamed_files
            .borrow_mut()
            .push((prev.to_path_buf(), next.to_path_buf()));
        Ok(())
    }
}
