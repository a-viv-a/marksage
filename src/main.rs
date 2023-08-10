mod archive;
mod md_file;
mod util;

use std::path::PathBuf;

use archive::archive;
use clap::{Parser, Subcommand};

fn parse_path(arg: &str) -> Result<PathBuf, std::io::Error> {
    let path = PathBuf::from(arg);
    match path.try_exists() {
        Ok(true) => Ok(path),
        Ok(false) => Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Path not found".to_string(),
        )),
        Err(e) => Err(e),
    }
}

#[derive(Parser, Debug)]
#[command(author, about, version)]
struct Cli {
    /// The path to the obsidian vault to operate on
    #[arg(short, long)]
    #[clap(value_parser = parse_path)]
    vault_path: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Archive todos that have been entirely completed
    Archive {},
}

fn main() {
    let args = Cli::parse();
    println!("Vault Path: {}", args.vault_path.display());
    println!("Command: {:?}", args.command);

    match args.command {
        Commands::Archive {} => {
            archive(args.vault_path);
        }
    }
}
