use crate::commands::collect::connect_database;
use core::exif::ExifMetadata;
use eyre::{eyre, Result};
use sqlx::sqlite::SqliteRow;
use sqlx::{FromRow, Row};
use std::collections::HashSet;
use std::io::Write;
use std::path::PathBuf;

struct DbFile(ExifMetadata);

impl<'r> FromRow<'r, SqliteRow> for DbFile {
    fn from_row(row: &'r SqliteRow) -> sqlx::Result<Self> {
        Ok(Self(ExifMetadata {
            source_file: row.try_get("source_file")?,
            file_name: row.try_get("file_name")?,
            file_size: row.try_get("file_size")?,
            file_type: row.try_get("file_type").map(|x: Option<String>| x)?,
            file_type_extension: row
                .try_get("file_type_extension")
                .map(|x: Option<String>| x)?,
            image_width: row
                .try_get("image_width")
                .map(|x: Option<u64>| x.map(|y| y as usize))?,
            date_time_original: row.try_get("date_time_original").map(|x: Option<String>| {
                match chrono::NaiveDateTime::parse_from_str(
                    x.unwrap_or("".into()).as_str(),
                    "%Y:%m:%d %H:%M:%S",
                ) {
                    Ok(dt) => Some(dt),
                    Err(_) => None,
                }
            })?,
            creation_date: row.try_get("creation_date").map(|x: Option<String>| {
                match chrono::NaiveDateTime::parse_from_str(
                    x.unwrap_or("".into()).as_str(),
                    "%Y:%m:%d %H:%M:%S",
                ) {
                    Ok(dt) => Some(dt),
                    Err(_) => None,
                }
            })?,
            ..Default::default()
        }))
    }
}

#[tokio::main]
pub async fn exec(db: &PathBuf) -> Result<()> {
    let time = std::time::Instant::now();

    println!("Finding duplicates...");
    let pool = connect_database(db, false).await?;

    let mut set: HashSet<ExifMetadata> = HashSet::new();
    let mut dups = vec![];

    let count: i64 = sqlx::query_as("select count(1) from files")
        .fetch_one(&pool)
        .await
        .map_err(|e| eyre!("{e}"))
        .map(|(count,)| count)?;
    let mut cursor = 1;

    while cursor < count {
        let data: Vec<DbFile> = sqlx::query_as("select * from files where id >= ?1 limit 100")
            .bind(cursor)
            .fetch_all(&pool)
            .await
            .map_err(|e| eyre!("{e}"))?;
        let round_count: i64 = data.len().try_into().unwrap();

        for item in data {
            let item = item.0;
            if let Some(dup) = set.get(&item) {
                dups.push((dup.source_file.clone(), item.source_file));
            } else {
                set.insert(item);
            }
        }
        cursor += round_count;
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
