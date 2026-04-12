use anyhow::Result;
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use renpyfmt::project::{FormatMode, format_directory, parse_directory};
use std::path::PathBuf;
use std::process::ExitCode;

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

        /// Check if files are formatted without modifying them.
        #[arg(long = "check")]
        check: bool,
    },
}

enum CommandOutcome {
    Success,
    CheckFailed,
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

fn run_format(path: PathBuf, config: Option<PathBuf>, check: bool) -> Result<CommandOutcome> {
    let pb = create_progress_bar();
    pb.set_message(if check {
        "checking format..."
    } else {
        "formatting..."
    });

    let report = format_directory(
        path,
        config,
        if check {
            FormatMode::Check
        } else {
            FormatMode::Write
        },
        pb,
    )?;

    if check && report.has_changes() {
        Ok(CommandOutcome::CheckFailed)
    } else {
        Ok(CommandOutcome::Success)
    }
}

fn run_parse(path: PathBuf) -> Result<CommandOutcome> {
    let pb = create_progress_bar();
    pb.set_message("parsing...");
    parse_directory(path, pb)?;
    Ok(CommandOutcome::Success)
}

fn try_main() -> Result<CommandOutcome> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Parse { path } => run_parse(path),
        Commands::Format {
            path,
            config,
            check,
        } => run_format(path, config, check),
    }
}

fn main() -> ExitCode {
    match try_main() {
        Ok(CommandOutcome::Success) => ExitCode::SUCCESS,
        Ok(CommandOutcome::CheckFailed) => ExitCode::from(1),
        Err(err) => {
            eprintln!("{err:#}");
            ExitCode::from(2)
        }
    }
}
