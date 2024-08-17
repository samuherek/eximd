use core::exif::{get_exif_metadata, ExifMetadata};
use core::file::collect_files;
use core::utils;
use std::collections::HashSet;
use std::error::Error;
use std::io::Write;
use std::path::Path;

pub fn exec(path: &Path) -> Result<(), Box<dyn Error>> {
    let files = collect_files(path);
    println!(
        "Collecting file metadata in '{}'",
        utils::path_to_string(path)
    );
    let files = files
        .iter()
        .filter_map(|x| {
            let path = x.path();
            get_exif_metadata(path).or_else(|| {
                eprintln!("{:?} -> Could not parse exif metadata", path);
                None
            })
        })
        .collect::<Vec<_>>();
    println!("Finding duplicates...");
    let mut set: HashSet<ExifMetadata> = HashSet::new();
    let mut dups = vec![];

    for item in files {
        if let Some(dup) = set.get(&item) {
            dups.push((dup.source_file.clone(), item.source_file));
        } else {
            set.insert(item);
        }
    }

    let mut stdout = std::io::stdout();
    for item in dups.iter() {
        writeln!(stdout, "Possible duplicates::")?;
        writeln!(stdout, "{}", item.0)?;
        writeln!(stdout, "{}", item.1)?;
        writeln!(stdout, "::")?;
    }

    stdout.flush()?;

    println!("Found {} possible duplicates", dups.len());

    Ok(())
}
