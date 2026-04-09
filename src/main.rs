use anyhow::Result;
use clap::{Parser, Subcommand};
use renpyfmt::project::{format_directory, parse_directory};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "renpyfmt")]
#[command(about = "Parse and format Ren'Py script files")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Parse {
        /// Directory to search recursively for .rpy files.
        path: PathBuf,
    },
    Format {
        /// Directory to search recursively for .rpy files.
        path: PathBuf,
    },
}

fn run_format(path: PathBuf) -> Result<()> {
    format_directory(path)
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Parse { path } => parse_directory(path),
        Commands::Format { path } => run_format(path),
    }
}
