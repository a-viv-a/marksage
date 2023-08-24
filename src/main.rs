mod archive;
mod format_files;
mod markdown_file;
mod notify_conflicts;
mod util;

use std::path::PathBuf;

use crate::notify_conflicts::notify_conflicts;
use archive::archive;
use clap::{Parser, Subcommand};
use format_files::format_files;
use url::Url;

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

fn parse_url(arg: &str) -> Result<Url, url::ParseError> {
    let url = arg.to_string();
    Url::parse(&url)
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
    /// Apply basic formatting to all markdown files in the vault
    Format {},
    /// Use ntfy.sh to send a push notification about sync conflicts
    NotifyConflicts {
        /// The ntfy.sh url to send the notification to
        #[arg(short, long)]
        #[clap(value_parser = parse_url, default_value = "https://ntfy.sh")]
        ntfy_url: Url,
        /// The topic to send the notification to
        #[arg(short, long)]
        topic: String,
    },
}

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Archive {} => {
            archive(args.vault_path);
        }
        Commands::Format {} => {
            format_files(args.vault_path);
        }
        Commands::NotifyConflicts { ntfy_url, topic } => {
            notify_conflicts(&args.vault_path, ntfy_url, topic);
        }
    }
}
