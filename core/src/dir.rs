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

#[cfg(test)]
mod test {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_collect_files_with_file() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("testfile.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "This is a test file.").unwrap();

        // Test: Call the function with a file path
        let result = collect_files(&file_path);

        // Assert: The result should contain one file
        assert!(result.is_ok());
        let files = result.unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].src.value(), &file_path);
    }

    #[test]
    fn test_collect_files_with_directory() {
        // Setup: Create a temporary directory and multiple files within it
        let temp_dir = tempdir().unwrap();
        let file1_path = temp_dir.path().join("file1.txt");
        let file2_path = temp_dir.path().join("file2.txt");
        File::create(&file1_path).unwrap();
        File::create(&file2_path).unwrap();

        // Test: Call the function with a directory path
        let result = collect_files(temp_dir.path());

        // Assert: The result should contain both files
        assert!(result.is_ok());
        let files = result.unwrap();
        let paths: Vec<PathBuf> = files
            .into_iter()
            .map(|f| f.src.value().to_owned())
            .collect();
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&file1_path));
        assert!(paths.contains(&file2_path));
    }

    #[test]
    fn test_collect_files_with_invalid_path() {
        // Setup: Create a non-existent path
        let invalid_path = Path::new("invalid/path/to/nothing");

        // Test: Call the function with an invalid path
        let result = collect_files(&invalid_path);

        // Assert: The result should be an error
        assert!(result.is_err());
    }

    #[test]
    fn test_collect_files_with_nested_directories() {
        // Setup: Create a temporary directory with nested subdirectories and files
        let temp_dir = tempdir().unwrap();
        let subdir1 = temp_dir.path().join("subdir1");
        let subdir2 = subdir1.join("subdir2");
        std::fs::create_dir_all(&subdir2).unwrap();

        let file1_path = temp_dir.path().join("file1.txt");
        let file2_path = subdir1.join("file2.txt");
        let file3_path = subdir2.join("file3.txt");

        File::create(&file1_path).unwrap();
        File::create(&file2_path).unwrap();
        File::create(&file3_path).unwrap();

        // Test: Call the function with the root directory path
        let result = collect_files(temp_dir.path());

        // Assert: The result should contain all three files
        assert!(result.is_ok());
        let files = result.unwrap();
        let paths: Vec<PathBuf> = files
            .into_iter()
            .map(|f| f.src.value().to_owned())
            .collect();
        assert_eq!(paths.len(), 3);
        assert!(paths.contains(&file1_path));
        assert!(paths.contains(&file2_path));
        assert!(paths.contains(&file3_path));
    }
}
