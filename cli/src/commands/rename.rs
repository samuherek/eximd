use super::super::config::RunType;
use super::super::file_system::FileSystem;
use core::exif;
use core::file::FilePath;
use core::file::InputFile;
use core::utils;
use std::collections::HashMap;

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
    UncertainPriaryFile(Vec<FilePath>),
}

pub fn process_files<F: FileSystem>(fs: &F, files: &[InputFile]) {
    let cmd_path = "exiftool";
    let mut groups: HashMap<String, FileGroup> = HashMap::new();
    let mut errors: Vec<ProcessError> = Vec::new();

    for item in files {
        let g = groups.entry(item.hash_key()).or_insert(FileGroup::new());
        if utils::is_primary_ext(item.ext.value()) {
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
                let item = exif::get_exif_file_from_input(&cmd_path, item);
                if let Some(next_src) = item.next_file_src() {
                    match fs.rename(&item.src.value(), &next_src.as_path()) {
                        Ok(_) => {
                            println!(
                                "{} -> {}",
                                item.src.as_str(),
                                utils::path_to_string(&next_src)
                            );
                        }
                        Err(err) => {
                            eprintln!("{} -> {}", item.src.as_str(), err);
                        }
                    }
                } else {
                    println!("{} -> Did not find clear exif date", item.src.as_str())
                }
            }
        // 3. We have only the secondary files. We don't rename at this point.
        //      We might add this  in the future if we find it usefull.
        } else if prim_len == 0 && sec_len > 0 {
            for item in group.secondary {
                println!("{} -> Not a media file", item.src.as_str());
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
            let prim_file = exif::get_exif_file_from_input(&cmd_path, prim_file);
            if let Some(next_stem) = prim_file.next_file_stem() {
                let prim_prev_src = prim_file.src.value().as_path();
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
                        eprintln!("{} -> {}", prim_file.src.as_str(), err);
                        needs_rollback = true;
                    }
                }
                for item in group.secondary {
                    let item = exif::get_exif_file_from_input(&cmd_path, item);
                    let sec_prev_src = item.src.value().as_path();
                    let sec_next_file_src =
                        item.src
                            .with_file_name(format!("{}.{}", next_stem, item.ext.value()));
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
                                eprintln!("{} -> {}", prim_file.src.as_str(), err);
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
                    println!("{} -> Did not find clear exif date", item.src.as_str())
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
                    println!("{} -> Uncertain Primary file", path.as_str());
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
