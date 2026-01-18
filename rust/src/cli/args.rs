//! CLI argument definitions

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// MethodRay - Fast Ruby type checker
#[derive(Parser)]
#[command(name = "methodray")]
#[command(about = "Fast Ruby type checker with method chain validation", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Check Ruby file(s) for type errors
    Check {
        /// Ruby file to check (if not specified, checks all files in project)
        #[arg(value_name = "FILE")]
        file: Option<PathBuf>,

        /// Show detailed output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Watch a Ruby file and re-check on changes
    Watch {
        /// Ruby file to watch
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },

    /// Show version information
    Version,

    /// Clear RBS cache
    ClearCache,
}
