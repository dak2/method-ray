use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod types;
mod parser;
mod graph;
mod env;
mod analyzer;
mod rbs;
mod cache;
mod diagnostics;
mod source_map;
mod checker;

use checker::FileChecker;

/// MethodRay - Fast Ruby type checker
#[derive(Parser)]
#[command(name = "methodray")]
#[command(about = "Fast Ruby type checker with method chain validation", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { file, verbose } => {
            if let Some(file_path) = file {
                let success = check_single_file(&file_path, verbose)?;
                if !success {
                    std::process::exit(1);
                }
            } else {
                check_project(verbose)?;
            }
        }
        Commands::Watch { file } => {
            watch_file(&file)?;
        }
        Commands::Version => {
            println!("MethodRay {}", env!("CARGO_PKG_VERSION"));
        }
        Commands::ClearCache => {
            clear_cache()?;
        }
    }

    Ok(())
}

fn check_single_file(file_path: &PathBuf, verbose: bool) -> Result<bool> {
    let checker = FileChecker::new()?;
    let diagnostics = checker.check_file(file_path)?;

    if diagnostics.is_empty() {
        if verbose {
            println!("{}: No errors found", file_path.display());
        }
        Ok(true) // No errors
    } else {
        // Use format_diagnostics_with_file for code snippet display
        let output = diagnostics::format_diagnostics_with_file(&diagnostics, file_path);

        println!("{}", output);

        // Check if there are any errors
        let has_errors = diagnostics
            .iter()
            .any(|d| d.level == diagnostics::DiagnosticLevel::Error);

        Ok(!has_errors) // Return true if no errors (only warnings)
    }
}

fn check_project(_verbose: bool) -> Result<()> {
    println!("Project-wide checking not yet implemented");
    println!("Use: methodray check <file> to check a single file");
    Ok(())
}

fn watch_file(file_path: &PathBuf) -> Result<()> {
    use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
    use std::sync::mpsc::channel;
    use std::time::Duration;

    // Verify file exists
    if !file_path.exists() {
        anyhow::bail!("File not found: {}", file_path.display());
    }

    println!("Watching {} for changes (Press Ctrl+C to stop)", file_path.display());
    println!();

    // Initial check
    println!("Initial check:");
    let mut had_errors = match check_single_file(file_path, true) {
        Ok(success) => !success,
        Err(e) => {
            eprintln!("Error during initial check: {}", e);
            true
        }
    };
    println!();

    // Setup file watcher
    let (tx, rx) = channel();

    let mut watcher = RecommendedWatcher::new(
        move |res| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        },
        Config::default().with_poll_interval(Duration::from_millis(500)),
    )?;

    // Watch the file
    watcher.watch(file_path.as_ref(), RecursiveMode::NonRecursive)?;

    // Event loop
    loop {
        match rx.recv() {
            Ok(event) => {
                // Only re-check on modify events
                if let notify::EventKind::Modify(_) = event.kind {
                    println!("\n--- File changed, re-checking... ---\n");

                    // Small delay to ensure file is fully written
                    std::thread::sleep(Duration::from_millis(100));

                    match check_single_file(file_path, true) {
                        Ok(success) => {
                            if success && had_errors {
                                // Errors were fixed
                                println!("✓ All errors fixed!");
                                had_errors = false;
                            } else if !success && !had_errors {
                                // New errors appeared
                                had_errors = true;
                            } else if success && !had_errors {
                                // Still no errors
                                // Message already printed by check_single_file with verbose=true
                            }
                            // If still has errors (had_errors && !success), no additional message needed
                        }
                        Err(e) => {
                            eprintln!("Error during check: {}", e);
                            had_errors = true;
                        }
                    }
                    println!();
                }
            }
            Err(e) => {
                eprintln!("Watch error: {}", e);
                break;
            }
        }
    }

    Ok(())
}

fn clear_cache() -> Result<()> {
    use cache::RbsCache;

    match RbsCache::cache_path() {
        Ok(path) => {
            if path.exists() {
                std::fs::remove_file(&path)?;
                println!("Cache cleared: {}", path.display());
            } else {
                println!("No cache file found");
            }
        }
        Err(e) => {
            eprintln!("Failed to get cache path: {}", e);
        }
    }

    Ok(())
}
