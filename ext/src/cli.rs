//! MethodRay CLI entry point for gem distribution

use anyhow::Result;
use clap::Parser;
use methodray_core::cli::{commands, Cli, Commands};

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
            eprintln!("WARNING: 'watch' is deprecated and will be removed in a future version.");
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
