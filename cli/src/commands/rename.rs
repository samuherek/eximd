use super::super::config::RunType;
use core::exif::{self, ExifNotifier, FileNameGroup};
use core::file::{FilePath, InputFile};
use core::utils;
use std::path::Path;

struct ConsoleNotifier;

impl ConsoleNotifier {
    fn new() -> Self {
        Self {}
    }
}

impl ExifNotifier for ConsoleNotifier {
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

    fn unsupported(&self, src: &FilePath) -> () {
        println!("{} -> Unsupported file", src.as_str());
    }
}

pub fn process_files<F: core::config::FileSystem>(fs: &F, files: &[InputFile]) {
    let cmd_path = "exiftool";
    let nf = ConsoleNotifier::new();
    println!("");
    println!("-");
    for mut group in exif::group_same_name_files(files) {
        match &mut group {
            FileNameGroup::Image { image, .. } => {
                image.fetch_and_set_metadata(&cmd_path);
                if let Some(next_stem) = image.next_file_stem_from_exif() {
                    exif::rename_with_rollback(fs, &nf, group.merge_into_rename_refs(), &next_stem);
                }
                println!("-")
            }
            FileNameGroup::Video { video, .. } => {
                video.fetch_and_set_metadata(&cmd_path);
                if let Some(next_stem) = video.next_file_stem_from_exif() {
                    exif::rename_with_rollback(fs, &nf, group.merge_into_rename_refs(), &next_stem);
                }
                println!("-")
            }
            FileNameGroup::LiveImage { image, .. } => {
                image.fetch_and_set_metadata(&cmd_path);
                if let Some(next_stem) = image.next_file_stem_from_exif() {
                    exif::rename_with_rollback(fs, &nf, group.merge_into_rename_refs(), &next_stem);
                }
                println!("-")
            }
            FileNameGroup::Uncertain {
                primary, config, ..
            } => {
                let values = primary.iter().chain(config.iter()).collect::<Vec<_>>();
                for item in values {
                    nf.uncertain(&item.src)
                }
                println!("-")
            }
            FileNameGroup::Unsupported { config, .. } => {
                for item in config {
                    nf.unsupported(&item.src);
                    println!("-");
                }
            }
        }
    }
    println!("");
}

pub fn print_mode(mode: &RunType) {
    match mode {
        RunType::Dry => println!("DRY RUN:: run `rename --exec 'path/to' to commit"),
        _ => {}
    }
}
