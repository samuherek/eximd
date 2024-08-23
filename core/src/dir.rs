use super::file::{FilePath, InputFile};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn get_valid_walk_entry(entry: &walkdir::DirEntry) -> Option<PathBuf> {
    let path = entry.path();
    if path.is_file() {
        Some(path.to_path_buf())
    } else {
        None
    }
}

// Accept either a directory or a file path.
// If it is a file, it will return a vector of just one file.
// If it is a directory, it will walk the files and return
// all the files recursivelly.
pub fn collect_files(path: &Path) -> Result<Vec<InputFile>, String> {
    // We support direct path
    if path.is_file() {
        let files = vec![InputFile::new(&FilePath::new(path), path)];
        return Ok(files);
        // We support a directory and we walk all the paths.
    } else if path.is_dir() {
        let files = WalkDir::new(path)
            .into_iter()
            .filter_map(Result::ok)
            .filter_map(|x| get_valid_walk_entry(&x))
            .map(|x| InputFile::new(&FilePath::new(&x), path))
            .collect::<Vec<_>>();
        return Ok(files);
        // In case is a symlink or something, let's error
    } else {
        return Err(format!(
            "Error: path is neither a file niether a dir: {}",
            path.display().to_string()
        ));
    }
}
