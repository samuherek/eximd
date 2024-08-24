use std::path::Path;

#[derive(PartialEq, Copy, Clone)]
pub enum RunType {
    Dry,
    Exec,
}

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
        if self.mode == RunType::Exec {
            std::fs::rename(prev, next)?;
        }
        Ok(())
    }
}

