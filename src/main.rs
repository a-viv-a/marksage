mod archive;
#[cfg(feature = "dry_run")]
mod diff;
mod format_files;
mod markdown_file;
#[cfg(feature = "notify")]
mod notify_conflicts;
mod util;

use std::path::PathBuf;

#[cfg(feature = "dry_run")]
use crate::diff::diff;
use crate::markdown_file::File;
#[cfg(feature = "notify")]
use crate::notify_conflicts::notify_conflicts;
use archive::archive;
use clap::{Parser, Subcommand};
use format_files::format_files;
use rayon::prelude::ParallelIterator;
use std::io;
#[cfg(feature = "notify")]
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

#[cfg(feature = "notify")]
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

    /// Print what would be done without actually doing it
    #[arg(short, long, default_value = "false")]
    #[cfg(feature = "dry_run")]
    dry_run: bool,

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
    #[cfg(feature = "notify")]
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

#[cfg(feature = "dry_run")]
fn write_file(
    mut stdout_buffer: Vec<String>,
    arg: &Cli,
    path: PathBuf,
    content: String,
) -> (Vec<String>, io::Result<()>) {
    use std::fs;

    if arg.dry_run {
        (
            match fs::read_to_string(path) {
                Ok(old_content) => {
                    stdout_buffer
                        .push("  dry run, would make the following changes:\n".to_string());
                    diff(stdout_buffer, &old_content, &content)
                }
                Err(_) => {
                    stdout_buffer.push(format!(
                        "  dry run, couldn't read old file! new file would be:\n{}\n",
                        content
                    ));
                    stdout_buffer
                }
            },
            Ok(()),
        )
    } else {
        (stdout_buffer, File::atomic_overwrite(&path, content))
    }
}

#[cfg(not(feature = "dry_run"))]
fn write_file(
    stdout_buffer: Vec<String>,
    _arg: &Cli,
    path: PathBuf,
    content: String,
) -> (Vec<String>, io::Result<()>) {
    (stdout_buffer, File::atomic_overwrite(&path, content))
}

fn apply_changes(
    args: &Cli,
    iter: impl ParallelIterator<Item = (PathBuf, String)>,
    verb: &str,
) -> Option<i32> {
    iter.map(|(path, content)| {
        let mut stdout_buffer: Vec<String> = Vec::with_capacity(3);
        stdout_buffer.push(format!("{verb} {}\n", path.display()));
        write_file(stdout_buffer, &args, path, content)
    })
    .map(|(mut stdout_buffer, result)| {
        if let Err(e) = result {
            stdout_buffer.push(format!("Failed to apply changes: {}\n", e));
            eprintln!("{}", stdout_buffer.join(""));
            1
        } else {
            println!("{}", stdout_buffer.join(""));
            0
        }
    })
    .max()
}

fn main() {
    let args = Cli::parse();

    let exit_code = match args.command {
        Commands::Archive {} => apply_changes(&args, archive(&args.vault_path), "Archived"),
        Commands::Format {} => apply_changes(&args, format_files(&args.vault_path), "Formatted"),
        #[cfg(feature = "notify")]
        Commands::NotifyConflicts { ntfy_url, topic } => {
            notify_conflicts(&args.vault_path, ntfy_url, topic)
        }
    }
    .unwrap_or(0);

    std::process::exit(exit_code);
}
