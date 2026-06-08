use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use renpyfmt::lsp;
use renpyfmt::project::{
    FormatInput, FormatMode, format_inputs, format_stdin_source, parse_directory,
};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
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
    Lsp,
    Format(FormatCommandArgs),
    Check(FormatCommandArgs),
}

#[derive(Args)]
struct FormatCommandArgs {
    /// Files or directories to format. Use `-` to read from stdin.
    #[arg(value_name = "PATHS", default_value = ".")]
    inputs: Vec<String>,

    /// Use this Ruff config file instead of auto-discovery.
    #[arg(long = "config")]
    config: Option<PathBuf>,

    /// Filename to associate with stdin input.
    #[arg(long = "stdin-filename")]
    stdin_filename: Option<PathBuf>,
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

fn parse_format_input(args: &FormatCommandArgs) -> Result<FormatInput> {
    let has_stdin = args.inputs.iter().any(|input| input == "-");

    if has_stdin {
        if args.inputs.len() != 1 {
            anyhow::bail!("`-` cannot be combined with other inputs");
        }

        let filename = args.stdin_filename.clone().ok_or_else(|| {
            anyhow::anyhow!("`--stdin-filename <PATH>` is required when input is `-`")
        })?;
        return Ok(FormatInput::Stdin { filename });
    }

    if args.stdin_filename.is_some() {
        anyhow::bail!("`--stdin-filename` is only supported when input is `-`");
    }

    Ok(FormatInput::Paths(
        args.inputs.iter().map(PathBuf::from).collect(),
    ))
}

fn stdin_config_base(filename: &Path) -> PathBuf {
    filename
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn run_format_like(args: FormatCommandArgs, mode: FormatMode) -> Result<CommandOutcome> {
    let input = parse_format_input(&args)?;

    match input {
        FormatInput::Paths(paths) => {
            let pb = create_progress_bar();
            pb.set_message(if mode == FormatMode::Check {
                "checking format..."
            } else {
                "formatting..."
            });

            let report = format_inputs(FormatInput::Paths(paths), args.config, mode, pb)?;

            if mode == FormatMode::Check && report.has_changes() {
                Ok(CommandOutcome::CheckFailed)
            } else {
                Ok(CommandOutcome::Success)
            }
        }
        FormatInput::Stdin { filename } => {
            let mut source = String::new();
            io::stdin().read_to_string(&mut source)?;
            let formatted = format_stdin_source(
                &source,
                filename.clone(),
                stdin_config_base(&filename),
                args.config,
            )?;

            if mode == FormatMode::Check {
                if source == formatted {
                    Ok(CommandOutcome::Success)
                } else {
                    eprintln!("Would reformat stdin");
                    Ok(CommandOutcome::CheckFailed)
                }
            } else {
                print!("{formatted}");
                Ok(CommandOutcome::Success)
            }
        }
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
        Commands::Lsp => {
            tokio::runtime::Runtime::new()?.block_on(lsp::run_server());
            Ok(CommandOutcome::Success)
        }
        Commands::Format(args) => run_format_like(args, FormatMode::Write),
        Commands::Check(args) => run_format_like(args, FormatMode::Check),
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
