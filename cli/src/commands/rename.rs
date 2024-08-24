use super::super::config::RunType;
use super::super::file_system::FileSystem;
use core::exif::{self, ExifFile, FileNameGroup};
use core::file::{FilePath, InputFile};
use core::utils;
use std::path::Path;

fn print_rename_success(old_path: &FilePath, next_path: &Path) {
    println!(
        "{} -> {}",
        old_path.as_str(),
        utils::path_to_string(next_path)
    );
}

fn print_rename_error(old_path: &FilePath, err: String) {
    eprintln!("{} -> {}", old_path.as_str(), err);
}

fn print_rollback_success(old: &Path, next: &FilePath) {
    println!(
        "{} -> {} (ROLLBACK)",
        next.as_str(),
        utils::path_to_string(old),
    );
}

fn rename_with_rollback<F: FileSystem>(fs: &F, items: Vec<&ExifFile>, next_src: &Path) {
    let mut processed = vec![];
    let mut needs_rollback = false;
    for file in items {
        if !needs_rollback {
            match fs.rename(&file.src.value(), next_src) {
                Ok(_) => {
                    print_rename_success(&file.src, next_src);
                    processed.push((&file.src, next_src));
                }
                Err(err) => {
                    print_rename_error(&file.src, err.to_string());
                    needs_rollback = true;
                }
            }
        }
    }

    if needs_rollback {
        for file in processed {
            match fs.rename(file.1, file.0.value()) {
                Ok(_) => {
                    print_rollback_success(file.1, file.0);
                }
                Err(err) => {
                    eprintln!(
                        "ERROR: rolling back the {}: {}",
                        utils::path_to_string(&file.1),
                        err
                    )
                }
            }
        }
    }
}

pub fn process_files<F: FileSystem>(fs: &F, files: &[InputFile]) {
    let cmd_path = "exiftool";
    for mut group in exif::group_same_name_files(files) {
        match &mut group {
            FileNameGroup::Image { image, .. } => {
                image.fetch_and_set_metadata(&cmd_path);
                if let Some(next_src) = image.next_file_src() {
                    rename_with_rollback(fs, group.merge_into_refs(), &next_src);
                }
            }
            FileNameGroup::Video { video, .. } => {
                video.fetch_and_set_metadata(&cmd_path);
                if let Some(next_src) = video.next_file_src() {
                    rename_with_rollback(fs, group.merge_into_refs(), &next_src);
                }
            }
            FileNameGroup::LiveImage { image, .. } => {
                image.fetch_and_set_metadata(&cmd_path);
                if let Some(next_src) = image.next_file_src() {
                    rename_with_rollback(fs, group.merge_into_refs(), &next_src);
                }
            }
            FileNameGroup::Uncertain(list) => {
                for item in list {
                    println!("{} -> Uncertain Primary file", item.src.as_str());
                }
            }
        }
    }
}

pub fn print_mode(mode: &RunType) {
    match mode {
        RunType::Dry => println!("DRY RUN:: run `rename --exec 'path/to' to commit"),
        _ => {}
    }
}
