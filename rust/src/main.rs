//! MethodRay CLI - Fast Ruby type checker

use anyhow::Result;
use clap::Parser;

mod analyzer;
mod cache;
mod checker;
mod cli;
mod diagnostics;
mod env;
mod graph;
mod parser;
mod rbs;
mod source_map;
mod types;

use cli::{commands, Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { file, verbose } => {
            if let Some(file_path) = file {
                let success = commands::check_single_file(&file_path, verbose)?;
                if !success {
                    std::process::exit(1);
                }
            } else {
                commands::check_project(verbose)?;
            }
        }
        Commands::Watch { file } => {
            commands::watch_file(&file)?;
        }
        Commands::Version => {
            commands::print_version();
        }
        Commands::ClearCache => {
            commands::clear_cache()?;
        }
    }

    Ok(())
}
