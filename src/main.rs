use anyhow::Result;
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
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

        /// Use this Ruff config file instead of auto-discovery.
        #[arg(long = "config")]
        config: Option<PathBuf>,
    },
}

fn create_progress_bar() -> ProgressBar {
    let pb = ProgressBar::new(0);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
        )
        .unwrap()
        .progress_chars("#>-"),
    );
    pb
}

fn run_format(path: PathBuf, config: Option<PathBuf>) -> Result<()> {
    let pb = create_progress_bar();
    pb.set_message("formatting...");
    format_directory(path, config, pb)
}

fn run_parse(path: PathBuf) -> Result<()> {
    let pb = create_progress_bar();
    pb.set_message("parsing...");
    parse_directory(path, pb)
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Parse { path } => run_parse(path),
        Commands::Format { path, config } => run_format(path, config),
    }
}
