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
                rename::RunType::Exec
            } else {
                rename::RunType::Dry
            };
            let path_buf = path.unwrap_or_else(|| {
                std::env::current_dir()
                    .expect("Did not provide path and couldn't read current dir.")
            });
            rename::complex_paths(path_buf, mode)?;
            // let path = path_buf.as_path();
            // println!("RENAME:: {:?}", path);
            //
            // if mode == rename::RunType::Dry {
            //     println!("MODE:: dry run");
            // }
            // rename::get_new_paths(path, mode)?;
        }
        _ => {
            println!("Incorrect usage");
        }
    }

    Ok(())
}
