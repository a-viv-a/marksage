use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, about, version)]
struct Cli {
    #[arg(short, long)]
    vault_path: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Archive {},
}

fn main() {
    let args = Cli::parse();
    println!("Vault Path: {}", args.vault_path);
    println!("Command: {:?}", args.command);
}
