use core::dir::collect_files;
use core::exif::{self, ExifMetadata};
use core::file::{FilePath, InputFile};
use core::utils;
use eyre::{eyre, Result};
use indicatif::{ProgressBar, ProgressStyle};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub async fn connect_database(path: &Path, force: bool) -> Result<SqlitePool> {
    println!("path {path:?}");
    if let Some(parent_dir) = path.parent() {
        if !parent_dir.exists() {
            fs::create_dir_all(parent_dir)?;
        }
    }

    let options = SqliteConnectOptions::from_str(path.to_str().unwrap())?.create_if_missing(true);
    let pool = SqlitePool::connect_with(options).await?;

    if force {
        sqlx::query("drop table if exists files")
            .execute(&pool)
            .await
            .map_err(|e| eyre!("Failed to migrate database {e}"))?;
    }

    sqlx::query(
        r#"
        create table if not exists files(
            id integer primary key, 
            source_file text not null,
            file_name text not null,
            file_size text not null,
            file_type text,
            file_type_extension text,
            image_width integer,
            date_time_original text,
            creation_date text
            )
    "#,
    )
    .execute(&pool)
    .await
    .map_err(|e| eyre!("Failed to migrate database {e}"))?;

    Ok(pool)
}

async fn save_to_database(exif_files: &Vec<ExifMetadata>, pool: &SqlitePool) -> Result<()> {
    for file in exif_files {
        sqlx::query(r#"
                insert into files(source_file, file_name, file_size, file_type, file_type_extension, image_width, date_time_original, creation_date) 
                values(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#)
                .bind(&file.source_file)
                .bind(&file.file_name)
                .bind(&file.file_size)
                .bind(&file.file_type)
                .bind(&file.file_type_extension)
                .bind(file.image_width.map(|x| x as i64))
                .bind(file.date_time_original.map(|x| x.to_string()))
                .bind(file.creation_date.map(|x| x.to_string()))
                .execute(pool)
                .await
                .map_err(|e| eyre!("Failed to save item to database {e}"))?;
    }

    Ok(())
}

fn get_files(path: &Path) -> Result<Vec<InputFile>> {
    collect_files(path).map_err(|x| eyre!("{x}"))
}

async fn collect_metadata(files: Vec<InputFile>, pool: &SqlitePool) -> Result<()> {
    let progress = ProgressBar::new(files.len().try_into()?).with_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} items")?,
    );

    let mut cursor = 0;
    let mut exif_buff = vec![];
    let step = 10;

    while cursor < files.len() {
        for (i, file) in files.iter().skip(cursor).take(step).enumerate() {
            if let Some(exif_file) = exif::get_exif_file_from_input(&"exiftool", file).metadata {
                exif_buff.push(exif_file);
            }
            progress.set_message(format!("Processing item {}", cursor + i));
            progress.set_position((cursor + i).try_into()?);
        }

        save_to_database(&exif_buff, &pool).await?;
        exif_buff.clear();
        cursor += step;
    }

    progress.finish_and_clear();

    Ok(())
}

// The full function to collect and print all the possible
// duplicates that were found.
#[tokio::main]
pub async fn exec(
    path: &PathBuf,
    db: &PathBuf,
    force: bool,
    file_cache: bool,
    skip: Option<usize>,
    limit: Option<usize>,
    exec: bool,
) -> Result<(), Box<dyn Error>> {
    let time = std::time::Instant::now();
    let path_string = utils::path_to_string(path);

    println!("Collecting file paths in '{}'", path_string);
    let mut files: Vec<InputFile> = if file_cache {
        println!("Reading file cache.");
        let buff = fs::read_to_string("file_list.txt")?;
        let data: Vec<InputFile> = serde_json::from_str(&buff)?;
        if let Some(skip) = skip {
            data.into_iter().skip(skip).collect()
        } else {
            data
        }
    } else {
        let files = get_files(path)?;
        let data = serde_json::to_string(&files)?;
        fs::write("file_list.txt", data.as_bytes())?;
        files
    };

    if let Some(limit) = limit {
        files = files.into_iter().take(limit).collect();
    }

    println!("File list data saved to file_list.txt");

    if !exec {
        println!("{:?}", files.first().unwrap());
        println!("next file: {:?}", files);
        std::process::exit(0);
    }

    println!("Collecting file metadata in '{}'", path_string);
    let pool = connect_database(db, force).await?;

    collect_metadata(files, &pool).await?;

    let duration = indicatif::HumanDuration(time.elapsed());
    println!("Saved in the {db:?}. Took {}", duration);

    Ok(())
}
