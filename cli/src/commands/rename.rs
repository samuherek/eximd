use super::super::config::RunType;
use core::exif;
use core::file::{FilePath, InputFile};
use core::utils;
use std::path::Path;

struct ConsoleNotifier;

impl ConsoleNotifier {
    fn new() -> Self {
        Self {}
    }
}

impl exif::ExifNotifier for ConsoleNotifier {
    fn rename_success(&self, prev: &FilePath, next: &Path) -> () {
        println!("{} -> {}", prev.as_str(), utils::path_to_string(next));
    }
    fn rename_error(&self, prev: &FilePath, err: String) -> () {
        eprintln!("{} -> {}", prev.as_str(), err);
    }

    fn rollback_success(&self, next: &Path, prev: &FilePath) -> () {
        println!(
            "{} -> {} (ROLLBACK)",
            utils::path_to_string(next),
            prev.as_str(),
        );
    }

    fn rollback_error(&self, next: &Path, err: String) -> () {
        eprintln!(
            "ERROR: rolling back the {}: {}",
            utils::path_to_string(next),
            err
        )
    }

    fn uncertain(&self, src: &FilePath) -> () {
        println!("{} -> Uncertain Primary file", src.as_str());
    }
}

pub fn process_files(files: &[InputFile]) {
    let cmd_path = "exiftool";
    let nf = ConsoleNotifier::new();
    for mut group in exif::group_same_name_files(files) {
        exif::fetch_and_set_form_group(&nf, &cmd_path, &mut group);
    }
}

pub fn print_mode(mode: &RunType) {
    match mode {
        RunType::Dry => println!("DRY RUN:: run `rename --exec 'path/to' to commit"),
        _ => {}
    }
}
