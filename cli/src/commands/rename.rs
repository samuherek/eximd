use super::super::config::RunType;
use super::super::file_system::FileSystem;
use chrono::NaiveDateTime;
use core::exif::{self, ExifMetadata};
use core::file::InputFile;
use core::utils;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct ExifDateFile {
    src: PathBuf,
    _stem: String,
    ext: String,
    date_time_original: Option<NaiveDateTime>,
    creation_date: Option<NaiveDateTime>,
}

impl ExifDateFile {
    fn new(file: &InputFile, info: &ExifMetadata) -> Self {
        Self {
            src: file.src.clone(),
            _stem: file.stem.clone(),
            ext: file.ext.clone(),
            date_time_original: info.date_time_original,
            creation_date: info.creation_date,
        }
    }

    fn next_file_stem(&self) -> Option<String> {
        // TODO: Parametize the format of the date?
        self.date_time_original
            .or(self.creation_date)
            .map(|date| date.format("%Y-%m-%d_%H.%M.%S").to_string())
    }

    fn next_file_name(&self) -> Option<String> {
        self.next_file_stem().map(|x| format!("{}.{}", x, self.ext))
    }

    fn next_file_src(&self) -> Option<PathBuf> {
        self.next_file_name()
            .map(|name| self.src.with_file_name(name))
    }
}

struct FileGroup<'a> {
    primary: Vec<&'a InputFile>,
    secondary: Vec<&'a InputFile>,
}

impl<'a> FileGroup<'a> {
    fn new() -> Self {
        Self {
            primary: Vec::new(),
            secondary: Vec::new(),
        }
    }

    fn push_primary(&mut self, file: &'a InputFile) {
        self.primary.push(file)
    }

    fn push_secondary(&mut self, file: &'a InputFile) {
        self.secondary.push(file)
    }
}

enum ProcessError {
    UncertainPriaryFile(Vec<PathBuf>),
}

fn get_exif_file(item: &InputFile) -> ExifDateFile {
    let exif_date = exif::get_exif_metadata(item.path()).unwrap_or_default();
    ExifDateFile::new(item, &exif_date)
}

pub fn process_files<F: FileSystem>(fs: &F, files: &[InputFile]) {
    let mut groups: HashMap<String, FileGroup> = HashMap::new();
    let mut errors: Vec<ProcessError> = Vec::new();

    for item in files {
        let g = groups.entry(item.hash_key()).or_insert(FileGroup::new());
        if utils::is_primary_ext(&item.ext) {
            g.push_primary(&item);
        } else {
            g.push_secondary(&item);
        }
    }

    // the cases we have
    for (_key, group) in groups {
        let prim_len = group.primary.len();
        let sec_len = group.secondary.len();

        // 1. In case we have more primary files with a possible date
        // and we have some secondary files as well, we don't konw
        // which "date_name" to choose. So we do nothing and report it
        // to the user. Otherwise, it's either just primary files
        // or it's
        if prim_len > 1 && sec_len > 0 {
            let prim = group.primary.iter().map(|x| x.src.clone());
            let sec = group.secondary.iter().map(|x| x.src.clone());
            let paths = prim.chain(sec).collect::<Vec<_>>();
            errors.push(ProcessError::UncertainPriaryFile(paths));
        // 2. we have only primary files and then we don't need a "rollback"
        //      and we can just rename one by one although they originally had the
        //      same name (we assume they had different extensions).
        } else if prim_len > 0 && sec_len == 0 {
            for item in group.primary {
                let item = get_exif_file(item);
                if let Some(next_src) = item.next_file_src() {
                    match fs.rename(&item.src.as_path(), &next_src.as_path()) {
                        Ok(_) => {
                            println!(
                                "{} -> {}",
                                utils::path_to_string(&item.src),
                                utils::path_to_string(&next_src)
                            );
                        }
                        Err(err) => {
                            eprintln!("{} -> {}", utils::path_to_string(&item.src), err);
                        }
                    }
                } else {
                    println!(
                        "{} -> Did not find clear exif date",
                        utils::path_to_string(&item.src)
                    )
                }
            }
        // 3. We have only the secondary files. We don't rename at this point.
        //      We might add this  in the future if we find it usefull.
        } else if prim_len == 0 && sec_len > 0 {
            for item in group.secondary {
                println!("{} -> Not a media file", utils::path_to_string(&item.src));
            }
        // 4. we have exactly one prim file and some secondary files and if something
        //      fails here, we need a rollback all the changes within this group
        } else if prim_len == 1 && sec_len > 0 {
            let mut processed = vec![];
            let mut needs_rollback = false;
            let prim_file = group
                .primary
                .get(0)
                .expect("At this point we need to have one exif file");
            let prim_file = get_exif_file(prim_file);
            if let Some(next_stem) = prim_file.next_file_stem() {
                let prim_prev_src = prim_file.src.as_path();
                let prim_next_file_src = prim_file
                    .next_file_src()
                    .expect("We already have a stem. We need to have the src");
                let prim_next_src = prim_next_file_src.as_path();
                match fs.rename(prim_prev_src, prim_next_src) {
                    Ok(_) => {
                        println!(
                            "{} -> {}",
                            utils::path_to_string(prim_prev_src),
                            utils::path_to_string(prim_next_src)
                        );
                        processed.push((prim_prev_src.to_path_buf(), prim_next_src.to_path_buf()));
                    }
                    Err(err) => {
                        eprintln!("{} -> {}", utils::path_to_string(&prim_file.src), err);
                        needs_rollback = true;
                    }
                }
                for item in group.secondary {
                    let item = get_exif_file(item);
                    let sec_prev_src = item.src.as_path();
                    let sec_next_file_src = item
                        .src
                        .with_file_name(format!("{}.{}", next_stem, item.ext));
                    let sec_next_src = sec_next_file_src.as_path();
                    if !needs_rollback {
                        match fs.rename(sec_prev_src, sec_next_src) {
                            Ok(_) => {
                                println!(
                                    "{} -> {}",
                                    utils::path_to_string(sec_prev_src),
                                    utils::path_to_string(sec_next_src)
                                );
                                processed
                                    .push((sec_prev_src.to_path_buf(), sec_next_src.to_path_buf()));
                            }
                            Err(err) => {
                                eprintln!("{} -> {}", utils::path_to_string(&prim_file.src), err);
                                needs_rollback = true;
                            }
                        }
                    }
                }

                if needs_rollback {
                    for file in processed {
                        match fs.rename(&file.1, &file.0) {
                            Ok(_) => {
                                println!(
                                    "{} -> {} (ROLLBACK)",
                                    utils::path_to_string(&file.1),
                                    utils::path_to_string(&file.0)
                                );
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
            } else {
                for item in group.primary.iter().chain(group.secondary.iter()) {
                    println!(
                        "{} -> Did not find clear exif date",
                        utils::path_to_string(&item.src)
                    )
                }
            }
        } else {
            unreachable!();
        }
    }

    for error in errors {
        match error {
            ProcessError::UncertainPriaryFile(paths) => {
                for path in paths {
                    println!("{} -> Uncertain Primary file", utils::path_to_string(&path));
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
