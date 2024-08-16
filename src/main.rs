mod exif;
mod hash_file;
mod rename;
mod utils;
use clap::{Parser, Subcommand};
use exival::file_system::RealFileSystem;
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
    Test,
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
        Some(Commands::Test) => {
            println!("Test success.");
            // exif::read_with_rs()?;
        }
        Some(Commands::Rename { exec, path }) => {
            let mode = if exec {
                exival::config::RunType::Exec
            } else {
                exival::config::RunType::Dry
            };
            let fs = RealFileSystem::new(&mode);
            let path_buf = path.unwrap_or_else(|| {
                std::env::current_dir()
                    .expect("Did not provide path and couldn't read current dir.")
            });
            let files = rename::collect_files(&path_buf);
            rename::print_mode(&mode);
            rename::process_files(&fs, &files);
        }
        _ => {
            println!("Incorrect usage");
        }
    }

    Ok(())
}
