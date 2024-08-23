use super::file::{FileExt, FilePath, FileStem, FileType, InputFile};
use chrono::NaiveDateTime;
use serde::Deserialize;
use std::error::Error;
use std::path::PathBuf;
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

#[derive(Debug, serde::Deserialize, Default)]
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

#[derive(Debug)]
pub struct ExifFile {
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
            src: file.src.clone(),
            src_relative: file.src_relative.clone(),
            stem: file.stem.clone(),
            ext: file.ext.clone(),
            file_type: file.file_type.clone(),
            metadata: Some(info),
        }
    }

    pub fn next_file_stem(&self) -> Option<String> {
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

    fn next_file_name(&self) -> Option<String> {
        self.next_file_stem()
            .map(|x| format!("{}.{}", x, self.ext.value()))
    }

    pub fn next_file_src(&self) -> Option<PathBuf> {
        self.next_file_name()
            .map(|name| self.src.with_file_name(name))
    }
}

impl From<InputFile> for ExifFile {
    fn from(file: InputFile) -> Self {
        Self {
            src: file.src,
            src_relative: file.src_relative,
            stem: file.stem,
            ext: file.ext,
            file_type: file.file_type,
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
