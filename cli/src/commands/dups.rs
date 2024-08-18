use core::exif::{get_exif_metadata, ExifMetadata};
use core::file::collect_files;
use core::utils;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashSet;
use std::error::Error;
use std::io::Write;
use std::path::Path;

// The full function to collect and print all the possible
// duplicates that were found.
pub fn exec(path: &Path) -> Result<(), Box<dyn Error>> {
    let time = std::time::Instant::now();
    let path_string = utils::path_to_string(path);
    println!("Collecting file paths in '{}'", path_string);
    let files = collect_files(path);

    println!("Collecting file metadata in '{}'", path_string);
    let progress = ProgressBar::new(files.len().try_into()?).with_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} items")?,
    );
    let mut exif_files = vec![];
    for (i, file) in files.iter().enumerate() {
        let path = file.path();
        if let Some(exif_file) = get_exif_metadata(path).or_else(|| {
            eprintln!("{:?} -> Could not parse exif metadata", path);
            None
        }) {
            exif_files.push(exif_file);
        }
        progress.set_message(format!("Processing item {}", i));
        progress.set_position(i.try_into()?);
    }
    progress.finish_and_clear();

    println!("Finding duplicates...");
    let mut set: HashSet<ExifMetadata> = HashSet::new();
    let mut dups = vec![];

    for item in exif_files {
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
    let duration = indicatif::HumanDuration(time.elapsed());

    println!("Found {} possible duplicates in {}", dups.len(), duration);

    Ok(())
}
