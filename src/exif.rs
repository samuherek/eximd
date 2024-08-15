use std::error::Error;
use std::io::{BufRead, Write};
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

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ExifData {
    source_file: String,
    file_name: String,
    image_width: Option<usize>,
    image_height: Option<usize>,
    file_size: String,
    file_type: Option<String>,
    file_type_extension: Option<String>,
    date_created: Option<String>,
    date_time_original: Option<String>,
    create_date: Option<String>,
}

impl std::hash::Hash for ExifData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.file_size.hash(state);
        self.file_type.hash(state);
        self.file_type_extension.hash(state);
        self.date_time_original.hash(state);
        self.image_width.hash(state);
    }
}

impl PartialEq for ExifData {
    fn eq(&self, other: &Self) -> bool {
        self.file_name == other.file_name
    }
}

impl Eq for ExifData {}

pub fn parse() -> Result<Vec<ExifData>, Box<dyn Error>> {
    let file = std::fs::File::open("output.json").expect("Did not find the output.json file");
    let reader = std::io::BufReader::new(file);
    let start = std::time::Instant::now();
    let mut count = 0;
    let mut obj_count = 0;
    let mut errors = vec![];
    let mut buffer = String::new();
    let mut items = vec![];

    for line in reader.lines() {
        let mut line = line.expect("The line should exist.");

        // We need to remove the very first array element. Otherwise it won't parse it correctly.
        if line.len() == 2 && line.starts_with("[{") {
            line.remove(0);
        }

        // We need to have a clean object string which requires to remove the "," at the end.
        if line.len() == 2 && line.starts_with("},") {
            line.pop();
        }

        // We need to remove the very last array char otherwise, it can't parse it.
        if line.len() == 2 && line.starts_with("}]") {
            line.pop();
        }

        // println!("lines are {line}");

        buffer.push_str(&line);
        count += 1;

        if line.len() == 1 && line.starts_with("}") {
            match serde_json::from_str::<ExifData>(&buffer) {
                Ok(value) => {
                    buffer.clear();
                    if let Some(ext) = value.file_type_extension.clone() {
                        let ext = ext.to_lowercase();
                        if super::utils::IMGS.contains(&ext.as_str()) {
                            obj_count += 1;
                            items.push(value);
                        }
                    }
                }
                Err(err) => {
                    errors.push((buffer.clone(), err));
                    buffer.clear();
                }
            }
        }
    }

    let duration = start.elapsed();
    println!("It took: {duration:?}");
    println!("Read {count} lines");
    println!("Total object is: {obj_count}");
    println!("Total error count is : {}", errors.len());
    println!("Err : {:?}", errors.first());

    let string_metadata_size = std::mem::size_of::<ExifData>();
    let vector_metadata_size = std::mem::size_of::<Vec<ExifData>>();
    let heap_size: usize = items.iter().map(|s| std::mem::size_of_val(s)).sum();
    let elements_size = items.len() * string_metadata_size + heap_size;
    let total_size = elements_size + vector_metadata_size;
    println!("We have this much in memory: {total_size}");

    Ok(items)
}

pub fn get_one_exif_input(data: &str) -> Result<String, Box<dyn Error>> {
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

pub fn find_duplicates() -> Result<(), Box<dyn Error>> {
    let data = parse()?;
    let mut set: std::collections::HashSet<ExifData> = std::collections::HashSet::new();
    let mut dups = vec![];
    let start = std::time::Instant::now();
    let dup_file = std::fs::File::create("duplicates.txt")?;
    let mut writer = std::io::BufWriter::new(dup_file);

    for item in data {
        // name and file size. Let's see what we get.
        if let Some(dup) = set.get(&item) {
            dups.push((dup.source_file.clone(), item.source_file));
        } else {
            set.insert(item);
        }
    }

    println!("we found this many duplicates: {}", dups.len());
    for item in dups.iter() {
        writeln!(writer, "{}", item.0)?;
        writeln!(writer, "{}", item.1)?;
        writeln!(writer, "---")?;
    }

    writer.flush()?;

    let duration = start.elapsed();
    println!("time taken comparing stuff: {:?}", duration);

    Ok(())
}

pub fn get_key_map() -> Result<(), Box<dyn Error>> {
    let file = std::fs::File::open("output.json").expect("Did not find the output.json file");
    let reader = std::io::BufReader::new(file);
    let start = std::time::Instant::now();
    let mut buffer = String::new();
    let mut keys = std::collections::HashSet::new();

    for line in reader.lines() {
        let mut line = line.expect("The line should exist.");

        // We need to remove the very first array element. Otherwise it won't parse it correctly.
        if line.len() == 2 && line.starts_with("[{") {
            line.remove(0);
        }

        // We need to have a clean object string which requires to remove the "," at the end.
        if line.len() == 2 && line.starts_with("},") {
            line.pop();
        }

        // We need to remove the very last array char otherwise, it can't parse it.
        if line.len() == 2 && line.starts_with("}]") {
            line.pop();
        }

        // println!("lines are {line}");

        buffer.push_str(&line);

        if line.len() == 1 && line.starts_with("}") {
            match serde_json::from_str::<serde_json::Value>(&buffer) {
                Ok(value) => {
                    buffer.clear();
                    if let serde_json::Value::Object(map) = value {
                        for key in map.keys() {
                            keys.insert(key.clone());
                        }
                    }
                }
                Err(err) => {
                    println!("ERROR: {}", err);
                    buffer.clear();
                }
            }
        }
    }

    for key in keys {
        println!("{}", key);
    }

    let duration = start.elapsed();
    println!("It took: {duration:?}");

    // let string_metadata_size = std::mem::size_of::<ExifData>();
    // let vector_metadata_size = std::mem::size_of::<Vec<ExifData>>();
    // let heap_size: usize = items.iter().map(|s| std::mem::size_of_val(s)).sum();
    // let elements_size = items.len() * string_metadata_size + heap_size;
    // let total_size = elements_size + vector_metadata_size;
    // println!("We have this much in memory: {total_size}");

    Ok(())
}
