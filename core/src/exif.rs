use super::config::FileSystem;
use super::file::{FileExt, FilePath, FileStem, FileType, InputFile};
use super::utils;
use chrono::NaiveDateTime;
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;

// "GPSLatitude": "53 deg 12' 16.92\" N",
// "GPSLongitude": "6 deg 32' 13.56\" E",
// "GPSPosition": "53 deg 12' 16.92\" N, 6 deg 32' 13.56\" E"
// "GPSCoordinates": "53 deg 12' 16.56\" N, 6 deg 32' 13.92\" E, 0.337 m Below Sea Level",
// "GPSAltitude": "0.337 m",
// "GPSAltitudeRef": "Below Sea Level",
// "GPSLatitude": "53 deg 12' 16.56\" N",
// "GPSLongitude": "6 deg 32' 13.92\" E",
// "GPSPosition": "53 deg 12' 16.56\" N, 6 deg 32' 13.92\" E"
// "GPSCoordinates": "53 deg 12' 16.20\" N, 6 deg 32' 13.56\" E, 0.763 m Below Sea Level",
// "GPSAltitude": "0.763 m",
// "GPSAltitudeRef": "Below Sea Level",
// "GPSLatitude": "53 deg 12' 16.20\" N",
// "GPSLongitude": "6 deg 32' 13.56\" E",
// "GPSPosition": "53 deg 12' 16.20\" N, 6 deg 32' 13.56\" E"

#[derive(Debug, serde::Deserialize, Default, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct ExifMetadata {
    pub source_file: String,
    file_name: String,
    file_size: String,
    file_type: Option<String>,
    file_type_extension: Option<String>,
    image_width: Option<usize>,
    _image_height: Option<usize>,
    #[serde(default, deserialize_with = "parse_date")]
    _date_created: Option<NaiveDateTime>,
    #[serde(default, deserialize_with = "parse_date")]
    _create_date: Option<NaiveDateTime>,
    #[serde(default, deserialize_with = "parse_date")]
    pub date_time_original: Option<NaiveDateTime>,
    #[serde(default, deserialize_with = "parse_date")]
    pub creation_date: Option<NaiveDateTime>,
}

impl std::hash::Hash for ExifMetadata {
    // This hash function is needed in order to create a unieuq
    // hash key that represents possibly unique file exif data
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.file_size.hash(state);
        self.file_type.hash(state);
        self.file_type_extension.hash(state);
        self.date_time_original.hash(state);
        self.image_width.hash(state);
    }
}

impl PartialEq for ExifMetadata {
    fn eq(&self, other: &Self) -> bool {
        self.file_name == other.file_name
    }
}

impl Eq for ExifMetadata {}

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
        let s = if s.len() >= 18 { &s[..19] } else { &s };
        match chrono::NaiveDateTime::parse_from_str(&s, "%Y:%m:%d %H:%M:%S") {
            Ok(dt) => Ok(Some(dt)),
            Err(_) => Ok(None),
        }
    } else {
        Ok(None)
    }
}

#[derive(Debug, Clone)]
pub struct ExifFile {
    pub group_key: String,
    pub src: FilePath,
    pub src_relative: FilePath,
    pub stem: FileStem,
    pub ext: FileExt,
    pub file_type: FileType,
    pub metadata: Option<ExifMetadata>,
}

impl ExifFile {
    fn new(file: &InputFile, info: ExifMetadata) -> Self {
        Self {
            group_key: file.hash_key(),
            src: file.src.clone(),
            src_relative: file.src_relative.clone(),
            stem: file.stem.clone(),
            ext: file.ext.clone(),
            file_type: file.file_type.clone(),
            metadata: Some(info),
        }
    }

    // TODO: These methods are kind of a mess. We need to look into it
    // again and see if we can clean it up in a more logical form.
    pub fn next_file_stem_from_exif(&self) -> Option<String> {
        // TODO: Parametize the format of the date
        self.metadata
            .as_ref()
            .map(|x| {
                x.date_time_original
                    .or(x.creation_date)
                    .map(|date| date.format("%Y-%m-%d_%H.%M.%S").to_string())
            })
            .flatten()
    }

    pub fn next_file_name(&self) -> Option<String> {
        self.next_file_stem_from_exif()
            .map(|x| format!("{}.{}", x, self.ext.value()))
    }

    pub fn next_file_src_from_exif(&self) -> Option<PathBuf> {
        self.next_file_name()
            .map(|name| self.src.with_file_name(name))
    }

    pub fn next_file_src_with_stem_name(&self, next_stem: &str) -> PathBuf {
        self.src
            .value()
            .with_file_name(format!("{}.{}", next_stem, self.ext.value()))
    }

    pub fn fetch_and_set_metadata(&mut self, cmd_path: &str) -> &Self {
        self.metadata = get_exif_metadata_from_cmd(cmd_path, &self.src);
        self
    }

    pub fn get_key(&self) -> String {
        self.stem.value().into()
    }
}

impl From<InputFile> for ExifFile {
    fn from(file: InputFile) -> Self {
        Self {
            group_key: file.hash_key(),
            src: file.src,
            src_relative: file.src_relative,
            stem: file.stem,
            ext: file.ext,
            file_type: file.file_type,
            metadata: None,
        }
    }
}

impl From<&InputFile> for ExifFile {
    fn from(file: &InputFile) -> Self {
        Self {
            group_key: file.hash_key(),
            src: file.src.clone(),
            src_relative: file.src_relative.clone(),
            stem: file.stem.clone(),
            ext: file.ext.clone(),
            file_type: file.file_type.clone(),
            metadata: None,
        }
    }
}

// When getting the data for each item from the exiftool and stdout
// it is passed as an array of objects serde does not automatically pares it.
// We take away all the wrapper stuff and return a valid object that can be
// serde paresd.
fn obj_str_from_array_of_one(data: &str) -> Result<String, Box<dyn Error>> {
    let mut buffer = String::new();

    for mut line in data.lines() {
        // We need to remove the very first array element. Otherwise it won't parse it correctly.
        if line.len() == 2 && line.starts_with("[{") {
            line = &line[1..];
        }

        // We need to have a clean object string which requires to remove the "," at the end.
        if line.len() == 2 && line.starts_with("},") {
            line = &line[0..(line.len() - 1)];
        }

        // We need to remove the very last array char otherwise, it can't parse it.
        if line.len() == 2 && line.starts_with("}]") {
            line = &line[0..(line.len() - 1)];
        }

        buffer.push_str(&line);

        if line.len() == 1 && line.starts_with("}") {
            return Ok(buffer);
        }
    }

    panic!("Should never get here.");
}

// This function runs the exiftool command which's path is passed
// as the cmd_path argument. And it will get the exif data
// and return it as a JSON object in a string.
// TODO: See if we need to return an error or doing these expects are ok
fn get_exif_metadata_from_cmd(cmd_path: &str, path: &FilePath) -> Option<ExifMetadata> {
    let cmd = Command::new(cmd_path)
        .args(["-j", path.as_str()])
        .output()
        .expect("tu run exiftool command");

    let data = String::from_utf8(cmd.stdout).expect("convert the utf8 into a string");
    let value = match obj_str_from_array_of_one(&data) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("Error: {}", err);
            return None;
        }
    };
    match serde_json::from_str::<ExifMetadata>(&value) {
        Ok(value) => Some(value),
        Err(err) => {
            eprintln!("Error: {}", err);
            None
        }
    }
}

// This is the primary function to run to get from input file to
// the actul file with dir info and metadata info.
// Get the exift data and merge them with the InputFile from the
// exiftool from the command line.
//
// TODO: Maybe we need to return a result? Maybe we need to notify the user somehow?
// But probably not. If we have "none" for the exif metadata, it's missing.
pub fn get_exif_file_from_input(cmd_path: &str, item: &InputFile) -> ExifFile {
    let data = get_exif_metadata_from_cmd(cmd_path, &item.src).unwrap_or_default();
    ExifFile::new(item, data)
}

#[derive(Debug, Eq, PartialEq, Clone, serde::Deserialize, serde::Serialize)]
pub struct FileNameGroupKey(String);

impl FileNameGroupKey {
    pub fn value(&self) -> &str {
        &self.0
    }
}

impl From<&str> for FileNameGroupKey {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<&InputFile> for FileNameGroupKey {
    fn from(file: &InputFile) -> Self {
        Self(file.hash_key())
    }
}

impl ToString for FileNameGroupKey {
    fn to_string(&self) -> String {
        self.value().to_owned()
    }
}

impl std::hash::Hash for FileNameGroupKey {
    // This hash function is needed in order to create a unieuq
    // hash key that represents possibly unique file exif data
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value().hash(state);
    }
}

#[derive(Debug, Clone)]
pub enum FileNameGroup {
    Image {
        key: FileNameGroupKey,
        image: ExifFile,
        config: Vec<ExifFile>,
    },
    LiveImage {
        key: FileNameGroupKey,
        image: ExifFile,
        video: ExifFile,
        config: Vec<ExifFile>,
    },
    Video {
        key: FileNameGroupKey,
        video: ExifFile,
        config: Vec<ExifFile>,
    },
    Uncertain {
        key: FileNameGroupKey,
        primary: Vec<ExifFile>,
        config: Vec<ExifFile>,
    },
    Unsupported {
        key: FileNameGroupKey,
        config: Vec<ExifFile>,
    },
}

impl FileNameGroup {
    // This method is used to get all the file paths
    // so that we can do the renaming of each file.
    pub fn merge_into_rename_refs(&self) -> Vec<&ExifFile> {
        let mut merged = Vec::new();
        match self {
            Self::Image { image, config, .. } => {
                merged.push(image);
                merged.extend(config.iter());
            }
            Self::LiveImage {
                image,
                config,
                video,
                ..
            } => {
                merged.push(image);
                merged.push(video);
                merged.extend(config.iter());
            }
            Self::Video { video, config, .. } => {
                merged.push(video);
                merged.extend(config.iter());
            }
            _ => {
                // We do nothing, here, because we don't know how to rename
            }
        }
        merged
    }

    pub fn group_key(&self) -> &FileNameGroupKey {
        match self {
            FileNameGroup::Image { key, .. } => key,
            FileNameGroup::Video { key, .. } => key,
            FileNameGroup::LiveImage { key, .. } => key,
            FileNameGroup::Uncertain { key, .. } => key,
            FileNameGroup::Unsupported { key, .. } => key,
        }
    }
}

pub fn group_same_name_files(files: &[InputFile]) -> Vec<FileNameGroup> {
    let mut groups: HashMap<FileNameGroupKey, (Vec<ExifFile>, Vec<ExifFile>)> = HashMap::new();

    for item in files {
        let g = groups
            .entry(FileNameGroupKey::from(item))
            .or_insert((Vec::new(), Vec::new()));
        if utils::is_primary_ext(item.ext.value()) {
            g.0.push(ExifFile::from(item));
        } else {
            g.1.push(ExifFile::from(item));
        }
    }

    let mut file_name_groups = Vec::new();

    // TODO: We have a couple of clones in the code below. There must be a way to
    // do it without cloning. Maybe I don't understand the exact working of the
    // rust manipulations.
    for (key, (primary_files, config_files)) in groups {
        match primary_files.len() {
            1 => {
                // 1. if we have a media vector length of exactly 1:
                assert_eq!(primary_files.len(), 1);
                let primary_file = &primary_files[0];
                let next = match primary_file.file_type {
                    //  - then: if it is an image -> Image
                    FileType::IMG => FileNameGroup::Image {
                        key,
                        image: primary_file.clone(),
                        config: config_files,
                    },
                    //  - then: if it is a video -> Video
                    FileType::VIDEO => FileNameGroup::Video {
                        key,
                        video: primary_file.clone(),
                        config: config_files,
                    },
                    _ => {
                        unreachable!(
                            "We have found a non primary files in the list of primary files"
                        );
                    }
                };

                file_name_groups.push(next);
            }
            2 => {
                // 2. if we have media vector length exactly 2:
                assert_eq!(primary_files.len(), 2);
                let [item1, item2] = &primary_files[..] else {
                    unreachable!(
                        "We found different number of primary files in a vector. Logic bug"
                    );
                };
                let types = (&item1.file_type, &item2.file_type);

                match types {
                    //  - then: if we have exactly one image and one video -> LivePhoto
                    (FileType::IMG, FileType::VIDEO) | (FileType::VIDEO, FileType::IMG) => {
                        let (image, video) = match types {
                            (FileType::IMG, FileType::VIDEO) => (item1, item2),
                            (FileType::VIDEO, FileType::IMG) => (item2, item1),
                            _ => {
                                unreachable!("This must be a language error bug. As this should never happen.");
                            }
                        };
                        file_name_groups.push(FileNameGroup::LiveImage {
                            key,
                            image: image.clone(),
                            video: video.clone(),
                            config: config_files,
                        });
                    }
                    //  - otherwise: Uncertainty of all the related files
                    _ => {
                        file_name_groups.push(FileNameGroup::Uncertain {
                            key,
                            primary: primary_files,
                            config: config_files,
                        });
                    }
                }
            }
            _ => {
                // 3. if we have a media vector length anything else:
                if primary_files.len() > 0 {
                    file_name_groups.push(FileNameGroup::Uncertain {
                        key,
                        primary: primary_files,
                        config: config_files,
                    });
                } else {
                    file_name_groups.push(FileNameGroup::Unsupported {
                        key,
                        config: config_files,
                    });
                }
            }
        }
    }

    file_name_groups
}

pub trait ExifNotifier {
    fn rename_success(&self, prev: &FilePath, next: &Path) -> ();
    fn rename_error(&self, prev: &FilePath, err: String) -> ();
    fn rollback_success(&self, next: &Path, prev: &FilePath) -> ();
    fn rollback_error(&self, next: &Path, err: String) -> ();
    fn uncertain(&self, src: &FilePath) -> ();
    fn unsupported(&self, src: &FilePath) -> ();
}

pub fn rename_with_rollback<F: FileSystem, N: ExifNotifier>(
    fs: &F,
    nf: &N,
    items: Vec<&ExifFile>,
    next_stem: &str,
) -> usize {
    let mut processed = vec![];
    let mut needs_rollback = false;
    for file in items {
        if !needs_rollback {
            let next_src = file.next_file_src_with_stem_name(next_stem);
            match fs.rename(&file.src.value(), &next_src) {
                Ok(_) => {
                    nf.rename_success(&file.src, &next_src);
                    processed.push((&file.src, next_src));
                }
                Err(err) => {
                    nf.rename_error(&file.src, err.to_string());
                    needs_rollback = true;
                }
            }
        }
    }

    if needs_rollback {
        for file in processed.iter() {
            match fs.rename(&file.1, file.0.value()) {
                Ok(_) => {
                    nf.rollback_success(&file.1, file.0);
                }
                Err(err) => {
                    nf.rollback_error(&file.1, err.to_string());
                }
            }
        }
    }

    processed.len()
}

#[cfg(test)]
mod test {
    use super::super::config::MockFileSystem;
    use super::*;
    use chrono::NaiveDateTime;
    use serde_json;
    use std::path::Path;

    const DATE_FORMAT: &'static str = "%Y:%m:%d %H:%M:%S";

    #[test]
    fn test_parse_date_with_date_and_time() {
        let json_data = r#"{
            "SourceFile": "test.jpg",
            "FileName": "test.jpg",
            "FileSize": "1",
            "DateTimeOriginal": "2021:10:10 12:34:56"
        }"#;

        let metadata: ExifMetadata = serde_json::from_str(json_data).unwrap();

        assert_eq!(
            metadata.date_time_original,
            Some(NaiveDateTime::parse_from_str("2021:10:10 12:34:56", DATE_FORMAT).unwrap())
        );
    }

    #[test]
    fn test_parse_date_with_date_and_time_and_zone() {
        let json_data = r#"{
            "SourceFile": "test.jpg",
            "FileName": "test.jpg",
            "FileSize": "1",
            "DateTimeOriginal": "2022:03:17 17:37:48+01:00"
        }"#;

        let metadata: ExifMetadata = serde_json::from_str(json_data).unwrap();

        assert_eq!(
            metadata.date_time_original,
            Some(NaiveDateTime::parse_from_str("2022:03:17 17:37:48", DATE_FORMAT).unwrap())
        );
    }

    #[test]
    fn test_parse_date_with_date_only() {
        let json_data = r#"{
            "SourceFile": "test.jpg",
            "FileName": "test.jpg",
            "FileSize": "1",
            "DateTimeOriginal": "2022:03:17"
        }"#;

        let metadata: ExifMetadata = serde_json::from_str(json_data).unwrap();

        assert_eq!(metadata.date_time_original, None);
    }

    #[test]
    fn test_parse_date_with_no_date() {
        let json_data = r#"{
            "SourceFile": "test.jpg",
            "FileName": "test.jpg",
            "FileSize": "1"
        }"#;

        let metadata: ExifMetadata = serde_json::from_str(json_data).unwrap();

        assert_eq!(metadata.date_time_original, None);
    }

    #[test]
    fn create_exif_file_from_input_file() {
        let input_file = InputFile::new(
            &FilePath::new(Path::new("path/to/file.jpg")),
            Path::new("path"),
        );

        let exif_file = ExifFile::from(&input_file);

        assert_eq!(exif_file.src, input_file.src);
        assert_eq!(exif_file.src_relative, input_file.src_relative);
        assert_eq!(exif_file.stem, input_file.stem);
        assert_eq!(exif_file.ext, input_file.ext);
        assert_eq!(exif_file.file_type, input_file.file_type);
        assert_eq!(exif_file.metadata, None);
    }

    #[test]
    fn next_file_stem_from_exif_file_with_date_time_original() {
        let metadata = ExifMetadata {
            date_time_original: Some(
                NaiveDateTime::parse_from_str("2021:10:10 12:34:56", DATE_FORMAT).unwrap(),
            ),
            ..Default::default()
        };

        let exif_file = ExifFile::new(
            &InputFile::new(
                &FilePath::new(Path::new("path/to/file.jpg")),
                Path::new("path"),
            ),
            metadata,
        );

        assert_eq!(
            exif_file.next_file_stem_from_exif(),
            Some("2021-10-10_12.34.56".to_string())
        );
    }

    #[test]
    fn next_file_stem_from_exif_file_with_creation_date() {
        let metadata = ExifMetadata {
            creation_date: Some(
                NaiveDateTime::parse_from_str("2021:10:10 12:34:56", DATE_FORMAT).unwrap(),
            ),
            ..Default::default()
        };

        let exif_file = ExifFile::new(
            &InputFile::new(
                &FilePath::new(Path::new("path/to/file.jpg")),
                Path::new("path"),
            ),
            metadata,
        );

        assert_eq!(
            exif_file.next_file_stem_from_exif(),
            Some("2021-10-10_12.34.56".to_string())
        );
    }

    #[test]
    fn next_file_name_from_exif_file() {
        let metadata = ExifMetadata {
            creation_date: Some(
                chrono::NaiveDateTime::parse_from_str("2021:10:10 12:34:56", "%Y:%m:%d %H:%M:%S")
                    .unwrap(),
            ),
            ..Default::default()
        };

        let exif_file = ExifFile::new(
            &InputFile::new(
                &FilePath::new(Path::new("path/to/file.jpg")),
                Path::new("path"),
            ),
            metadata,
        );

        assert_eq!(
            exif_file.next_file_name(),
            Some("2021-10-10_12.34.56.jpg".to_string())
        );
    }

    #[test]
    fn group_same_name_files_one_image() {
        let input_files = vec![InputFile::new(
            &FilePath::new(Path::new("path/to/file.jpg")),
            Path::new("path"),
        )];

        let groups = group_same_name_files(&input_files);

        assert_eq!(groups.len(), 1);

        match &groups[0] {
            FileNameGroup::Image { key, image, config } => {
                assert_eq!(key.value(), "file");
                assert_eq!(image.ext.value(), "jpg");
                assert_eq!(config.len(), 0);
            }
            _ => panic!("Unexpected group type"),
        }
    }

    #[test]
    fn group_same_name_files_one_image_and_config_files() {
        let input_files = vec![
            InputFile::new(
                &FilePath::new(Path::new("path/to/file.jpg")),
                Path::new("path"),
            ),
            InputFile::new(
                &FilePath::new(Path::new("path/to/file.xml")),
                Path::new("path"),
            ),
            InputFile::new(
                &FilePath::new(Path::new("path/to/file.aea")),
                Path::new("path"),
            ),
        ];

        let groups = group_same_name_files(&input_files);

        assert_eq!(groups.len(), 1);

        match &groups[0] {
            FileNameGroup::Image { key, image, config } => {
                assert_eq!(key.value(), "file");
                assert_eq!(image.ext.value(), "jpg");
                assert_eq!(config.len(), 2);
            }
            _ => panic!("Unexpected group type"),
        }
    }

    #[test]
    fn group_same_name_files_one_video() {
        let input_files = vec![InputFile::new(
            &FilePath::new(Path::new("path/to/file.mov")),
            Path::new("path"),
        )];

        let groups = group_same_name_files(&input_files);

        assert_eq!(groups.len(), 1);

        match &groups[0] {
            FileNameGroup::Video { key, video, config } => {
                assert_eq!(key.value(), "file");
                assert_eq!(video.ext.value(), "mov");
                assert_eq!(config.len(), 0);
            }
            _ => panic!("Unexpected group type"),
        }
    }

    #[test]
    fn group_same_name_files_one_video_and_config_files() {
        let input_files = vec![
            InputFile::new(
                &FilePath::new(Path::new("path/to/file.xml")),
                Path::new("path"),
            ),
            InputFile::new(
                &FilePath::new(Path::new("path/to/file.mov")),
                Path::new("path"),
            ),
        ];

        let groups = group_same_name_files(&input_files);

        assert_eq!(groups.len(), 1);

        match &groups[0] {
            FileNameGroup::Video { key, video, config } => {
                assert_eq!(key.value(), "file");
                assert_eq!(video.ext.value(), "mov");
                assert_eq!(config.len(), 1);
            }
            _ => panic!("Unexpected group type"),
        }
    }

    #[test]
    fn group_same_name_files_no_media() {
        let input_files = vec![
            InputFile::new(
                &FilePath::new(Path::new("path/to/file.aea")),
                Path::new("path"),
            ),
            InputFile::new(
                &FilePath::new(Path::new("path/to/file.xml")),
                Path::new("path"),
            ),
        ];

        let groups = group_same_name_files(&input_files);

        assert_eq!(groups.len(), 1);

        match &groups[0] {
            FileNameGroup::Unsupported { key, config } => {
                assert_eq!(key.value(), "file");
                assert_eq!(config.len(), 2);
            }
            _ => panic!("Unexpected group type"),
        }
    }

    #[test]
    fn group_same_name_files_live_photo_with_video() {
        let input_files = vec![
            InputFile::new(
                &FilePath::new(Path::new("path/to/file.jpg")),
                Path::new("path"),
            ),
            InputFile::new(
                &FilePath::new(Path::new("path/to/file.mov")),
                Path::new("path"),
            ),
        ];

        let groups = group_same_name_files(&input_files);

        assert_eq!(groups.len(), 1);

        match &groups[0] {
            FileNameGroup::LiveImage {
                key,
                image,
                video,
                config,
            } => {
                assert_eq!(key.value(), "file");
                assert_eq!(image.ext.value(), "jpg");
                assert_eq!(video.ext.value(), "mov");
                assert_eq!(config.len(), 0);
            }
            _ => panic!("Unexpected group type"),
        }
    }

    struct MockExifNotifer;
    impl MockExifNotifer {
        fn new() -> Self {
            Self {}
        }
    }
    impl ExifNotifier for MockExifNotifer {
        fn rename_success(&self, _prev: &FilePath, _next: &Path) -> () {}
        fn rename_error(&self, _prev: &FilePath, _err: String) -> () {}
        fn rollback_success(&self, _next: &Path, _prev: &FilePath) -> () {}
        fn rollback_error(&self, _next: &Path, _err: String) -> () {}
        fn uncertain(&self, _src: &FilePath) -> () {}
        fn unsupported(&self, _src: &FilePath) -> () {}
    }

    #[test]
    fn rename_with_rollback_one_image() {
        let fs = MockFileSystem::new();
        let nf = MockExifNotifer::new();
        let image = ExifFile::new(
            &InputFile::new(
                &FilePath::new(Path::new("path/to/file.jpg")),
                Path::new("path"),
            ),
            ExifMetadata {
                ..Default::default()
            },
        );
        let files = vec![&image];
        let next_stem = "2021-10-10_12.34.56";

        rename_with_rollback(&fs, &nf, files, &next_stem);
        let renamed_files = fs.renamed_files.borrow();
        let first = renamed_files.first().unwrap();

        assert_eq!(renamed_files.len(), 1);
        assert_eq!(first.0, PathBuf::from("path/to/file.jpg"));
        assert_eq!(first.1, PathBuf::from("path/to/2021-10-10_12.34.56.jpg"));
    }

    #[test]
    fn rename_with_rollback_one_image_with_config_file() {
        let fs = MockFileSystem::new();
        let nf = MockExifNotifer::new();
        let image = ExifFile::new(
            &InputFile::new(
                &FilePath::new(Path::new("path/to/file.jpg")),
                Path::new("path"),
            ),
            ExifMetadata {
                ..Default::default()
            },
        );
        let config = ExifFile::new(
            &InputFile::new(
                &FilePath::new(Path::new("path/to/file.xml")),
                Path::new("path"),
            ),
            ExifMetadata {
                ..Default::default()
            },
        );

        let files = vec![&image, &config];
        let next_stem = "2021-10-10_12.34.56";

        rename_with_rollback(&fs, &nf, files, &next_stem);
        let renamed_files = fs.renamed_files.borrow();
        let first = renamed_files.get(0).unwrap();
        let second = renamed_files.get(1).unwrap();

        assert_eq!(renamed_files.len(), 2);
        assert_eq!(first.0, PathBuf::from("path/to/file.jpg"));
        assert_eq!(first.1, PathBuf::from("path/to/2021-10-10_12.34.56.jpg"));
        assert_eq!(second.0, PathBuf::from("path/to/file.xml"));
        assert_eq!(second.1, PathBuf::from("path/to/2021-10-10_12.34.56.xml"));
    }

    #[test]
    fn rename_with_rollback_one_image_with_config_files() {
        let fs = MockFileSystem::new();
        let nf = MockExifNotifer::new();
        let image = ExifFile::new(
            &InputFile::new(
                &FilePath::new(Path::new("path/to/file.jpg")),
                Path::new("path"),
            ),
            ExifMetadata {
                ..Default::default()
            },
        );
        let config1 = ExifFile::new(
            &InputFile::new(
                &FilePath::new(Path::new("path/to/file.xml")),
                Path::new("path"),
            ),
            ExifMetadata {
                ..Default::default()
            },
        );
        let config2 = ExifFile::new(
            &InputFile::new(
                &FilePath::new(Path::new("path/to/file.aae")),
                Path::new("path"),
            ),
            ExifMetadata {
                ..Default::default()
            },
        );

        let files = vec![&image, &config1, &config2];
        let next_stem = "2021-10-10_12.34.56";

        rename_with_rollback(&fs, &nf, files, &next_stem);
        let renamed_files = fs.renamed_files.borrow();
        let first = renamed_files.get(0).unwrap();
        let second = renamed_files.get(1).unwrap();
        let third = renamed_files.get(2).unwrap();

        assert_eq!(renamed_files.len(), 3);
        assert_eq!(first.0, PathBuf::from("path/to/file.jpg"));
        assert_eq!(first.1, PathBuf::from("path/to/2021-10-10_12.34.56.jpg"));
        assert_eq!(second.0, PathBuf::from("path/to/file.xml"));
        assert_eq!(second.1, PathBuf::from("path/to/2021-10-10_12.34.56.xml"));
        assert_eq!(third.0, PathBuf::from("path/to/file.aae"));
        assert_eq!(third.1, PathBuf::from("path/to/2021-10-10_12.34.56.aae"));
    }
}
