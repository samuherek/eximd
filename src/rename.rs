use super::exif;
use super::utils;
use exival::config::RunType;
use exival::file_system::FileSystem;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use walkdir::WalkDir;

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct DateTime {
    file_name: String,
    #[serde(default, deserialize_with = "parse_date")]
    date_time_original: Option<chrono::NaiveDateTime>,
    #[serde(default, deserialize_with = "parse_date")]
    creation_date: Option<chrono::NaiveDateTime>,
}

// We always want to take only the date and time from the string
// and ignore the miliseconds and the timezone information.
// When the exif data is created, the
// DateTimeOriginal -> is wihtout a time zone usually. If so the date and time is the
// date and time in the current time zone. So the timezone info is irrelevant.
// It looks like if we don't have a time zone in this tag, it will have the time
// and date of the timezone the media was taken in.
// CreationDate -> is with the time zone, but like the above, it will have the
// date and time in the current time zone time. So we can ignore the time zone.
//
// This way, we have the same date and time that is the date and time of the
// timezone that the media was taken and is relative time which is what we most likely want.
fn parse_date<'de, D>(deserializer: D) -> Result<Option<chrono::NaiveDateTime>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    if let Some(s) = s {
        let s = &s[..19];
        match chrono::NaiveDateTime::parse_from_str(&s, "%Y:%m:%d %H:%M:%S") {
            Ok(dt) => Ok(Some(dt)),
            Err(_) => Ok(None),
        }
    } else {
        Ok(None)
    }
}

fn exif_date_time<P: AsRef<Path>>(path: P) -> Option<DateTime> {
    let cmd = Command::new("exiftool")
        .args(["-j", path.as_ref().to_str().unwrap()])
        .output()
        .expect("Exiftool command did not work");

    let data = String::from_utf8(cmd.stdout).expect("To convert the utf8 into a string");
    let value = match exif::get_one_exif_input(&data) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("Error: {}", err);
            return None;
        }
    };
    match serde_json::from_str::<DateTime>(&value) {
        Ok(value) => Some(value),
        Err(err) => {
            eprintln!("Error: {}", err);
            None
        }
    }
}

struct FilePath(PathBuf);

impl FilePath {
    fn new(path: &Path) -> Self {
        Self(path.to_path_buf())
    }

    fn value(&self) -> &PathBuf {
        &self.0
    }
}

pub struct InputFile {
    src: PathBuf,
    stem: String,
    ext: String,
}

impl InputFile {
    fn new(file: &FilePath) -> Self {
        let src = file.value().to_owned();
        let stem = get_stem(&src);
        let ext = get_ext(&src);
        Self { src, stem, ext }
    }

    fn hash_key(&self) -> String {
        self.stem.clone()
    }

    fn path(&self) -> &PathBuf {
        &self.src
    }
}

fn get_stem(path: &Path) -> String {
    path.file_stem()
        .expect("To have a file stem")
        .to_string_lossy()
        .into()
}

fn get_ext(path: &Path) -> String {
    path.extension()
        .map(|i| i.to_string_lossy().into())
        .unwrap_or_default()
}

fn is_primary_ext(ext: &str) -> bool {
    utils::is_img(ext) || utils::is_video(ext)
}

// Walk the dir to collect all the files and ignore dirs -> We have a list of paths to check
pub fn collect_files(path: &Path) -> Vec<InputFile> {
    let mut files = vec![];
    // We support direct path
    if path.is_file() {
        let file = FilePath::new(path);
        files.push(InputFile::new(&file));
    // We support a directory and we walk all the paths.
    } else if path.is_dir() {
        for entry in WalkDir::new(path) {
            let entry = entry.map_or(None, |x| {
                let x_path = x.path();
                if x_path.is_file() {
                    Some(FilePath::new(x_path))
                } else {
                    None
                }
            });
            if let Some(entry) = entry {
                files.push(InputFile::new(&entry));
            }
        }
    } else {
        eprintln!(
            "Error: path is neither a file niether a dir: {}",
            path.display().to_string()
        );
    }

    files
}

pub struct ExifDateFile {
    src: PathBuf,
    stem: String,
    ext: String,
    date_time_original: Option<chrono::NaiveDateTime>,
    creation_date: Option<chrono::NaiveDateTime>,
}

impl ExifDateFile {
    fn new(file: &InputFile, info: &DateTime) -> Self {
        Self {
            src: file.src.clone(),
            stem: file.stem.clone(),
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

pub fn exif_date_files(files: &[InputFile]) -> Vec<ExifDateFile> {
    files
        .iter()
        .map(|file| {
            let exif_date = exif_date_time(file.path()).unwrap_or_default();
            ExifDateFile::new(file, &exif_date)
        })
        .collect()
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

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn get_exif_file(item: &InputFile) -> ExifDateFile {
    let exif_date = exif_date_time(item.path()).unwrap_or_default();
    ExifDateFile::new(item, &exif_date)
}

pub fn process_files<F: FileSystem>(fs: &F, files: &[InputFile]) {
    let mut groups: HashMap<String, FileGroup> = HashMap::new();
    let mut errors: Vec<ProcessError> = Vec::new();

    for item in files {
        let g = groups.entry(item.hash_key()).or_insert(FileGroup::new());
        if is_primary_ext(&item.ext) {
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
                                path_to_string(&item.src),
                                path_to_string(&next_src)
                            );
                        }
                        Err(err) => {
                            eprintln!("{} -> {}", path_to_string(&item.src), err);
                        }
                    }
                } else {
                    println!(
                        "{} -> Did not find clear exif date",
                        path_to_string(&item.src)
                    )
                }
            }
        // 3. We have only the secondary files. We don't rename at this point.
        //      We might add this  in the future if we find it usefull.
        } else if prim_len == 0 && sec_len > 0 {
            for item in group.secondary {
                println!("{} -> Not a media file", path_to_string(&item.src));
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
                            path_to_string(prim_prev_src),
                            path_to_string(prim_next_src)
                        );
                        processed.push((prim_prev_src.to_path_buf(), prim_next_src.to_path_buf()));
                    }
                    Err(err) => {
                        eprintln!("{} -> {}", path_to_string(&prim_file.src), err);
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
                                    path_to_string(sec_prev_src),
                                    path_to_string(sec_next_src)
                                );
                                processed
                                    .push((sec_prev_src.to_path_buf(), sec_next_src.to_path_buf()));
                            }
                            Err(err) => {
                                eprintln!("{} -> {}", path_to_string(&prim_file.src), err);
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
                                    path_to_string(&file.1),
                                    path_to_string(&file.0)
                                );
                            }
                            Err(err) => {
                                eprintln!(
                                    "ERROR: rolling back the {}: {}",
                                    path_to_string(&file.1),
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
                        path_to_string(&item.src)
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
                    println!("{} -> Uncertain Primary file", path_to_string(&path));
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
