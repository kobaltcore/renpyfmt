use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use renpyfmt::project::parse_directory;
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
    bail!("format is not implemented yet for {}", path.display())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Parse { path } => parse_directory(path),
        Commands::Format { path } => run_format(path),
    }
}
