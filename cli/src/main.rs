mod commands;

use clap::{Parser, Subcommand};
use commands::rename;
use core::config;
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
    Dups {
        path: Option<PathBuf>,
    },
    Rename {
        path: Option<PathBuf>,
        #[arg(short, long)]
        exec: bool,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Test) => {
            println!("Test success.");
            // exif::read_with_rs()?;
        }
        Some(Commands::Dups { path }) => {
            let path_buf = path.unwrap_or_else(|| {
                std::env::current_dir()
                    .expect("Did not provide path and couldn't read current dir.")
            });
            commands::dups::exec(&path_buf)?;
        }
        Some(Commands::Rename { exec, path }) => {
            let mode = if exec {
                config::RunType::Exec
            } else {
                config::RunType::Dry
            };
            let fs = config::RealFileSystem::new(&mode);
            let path_buf = path.unwrap_or_else(|| {
                std::env::current_dir()
                    .expect("Did not provide path and couldn't read current dir.")
            });
            let files = core::dir::collect_files(&path_buf)?;
            rename::print_mode(&mode);
            rename::process_files(&fs, &files);
            rename::print_mode(&mode);
        }
        _ => {
            println!("Incorrect usage");
        }
    }

    Ok(())
}
