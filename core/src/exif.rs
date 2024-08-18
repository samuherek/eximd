use chrono::NaiveDateTime;
use serde::Deserialize;
use std::error::Error;
use std::path::Path;
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

pub fn obj_str_from_array_of_one(data: &str) -> Result<String, Box<dyn Error>> {
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

pub fn get_exif_metadata<P: AsRef<Path>>(path: P) -> Option<ExifMetadata> {
    let cmd = Command::new("exiftool")
        .args(["-j", path.as_ref().to_str().unwrap()])
        .output()
        .expect("`exiftool` cli be available on the system");

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
