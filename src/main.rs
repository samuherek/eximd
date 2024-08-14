mod exif;
mod ext;
mod hash_file;
mod rename;
mod utils;
use clap::{Parser, Subcommand};
use std::error::Error;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Rename {
        path: Option<PathBuf>,
        #[arg(short, long)]
        exec: bool,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    // hash_file::calc_hashing()?;
    // exif::find_duplicates().expect("To just work.");
    // exif::get_key_map().expect("To just work.");
    // rename::rename("test_src/IMG_2213.DNG").expect("This to work");

    match cli.command {
        Some(Commands::Rename { exec, path }) => {
            let path_buf = path.unwrap_or_else(|| {
                std::env::current_dir()
                    .expect("Did not provide path and couldn't read current dir.")
            });
            let path = path_buf.as_path();
            println!("RENAME:: {:?}", path);

            if exec {
                println!("real run");
                // std::fs::rename(path, new_path)?;
                todo!();
            } else {
                println!("dry run");
                let next_paths = rename::get_new_paths(path, rename::RunType::Dry)?;
            }
            // println!("we are here");
            // rename::rename("test_src/IMG_2213.DNG").expect("This to work");
        }
        _ => {
            println!("Incorrect usage");
        }
    }

    Ok(())
}
