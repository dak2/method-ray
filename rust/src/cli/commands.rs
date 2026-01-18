//! CLI command implementations

use anyhow::Result;
use std::path::PathBuf;

use crate::cache::RbsCache;
use crate::checker::FileChecker;
use crate::diagnostics;

/// Check a single Ruby file for type errors
/// Returns Ok(true) if no errors, Ok(false) if errors found
pub fn check_single_file(file_path: &PathBuf, verbose: bool) -> Result<bool> {
    let checker = FileChecker::new()?;
    let diagnostics = checker.check_file(file_path)?;

    if diagnostics.is_empty() {
        if verbose {
            println!("{}: No errors found", file_path.display());
        }
        Ok(true)
    } else {
        let output = diagnostics::format_diagnostics_with_file(&diagnostics, file_path);
        println!("{}", output);

        let has_errors = diagnostics
            .iter()
            .any(|d| d.level == diagnostics::DiagnosticLevel::Error);

        Ok(!has_errors)
    }
}

/// Check all Ruby files in the project
pub fn check_project(_verbose: bool) -> Result<()> {
    println!("Project-wide checking not yet implemented");
    println!("Use: methodray check <file> to check a single file");
    Ok(())
}

/// Watch a file for changes and re-check on modifications
pub fn watch_file(file_path: &PathBuf) -> Result<()> {
    use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
    use std::sync::mpsc::channel;
    use std::time::Duration;

    if !file_path.exists() {
        anyhow::bail!("File not found: {}", file_path.display());
    }

    println!(
        "Watching {} for changes (Press Ctrl+C to stop)",
        file_path.display()
    );
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

    watcher.watch(file_path.as_ref(), RecursiveMode::NonRecursive)?;

    // Event loop
    loop {
        match rx.recv() {
            Ok(event) => {
                if let notify::EventKind::Modify(_) = event.kind {
                    println!("\n--- File changed, re-checking... ---\n");

                    std::thread::sleep(Duration::from_millis(100));

                    match check_single_file(file_path, true) {
                        Ok(success) => {
                            if success && had_errors {
                                println!("✓ All errors fixed!");
                                had_errors = false;
                            } else if !success && !had_errors {
                                had_errors = true;
                            }
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

/// Clear the RBS cache
pub fn clear_cache() -> Result<()> {
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

/// Print version information
pub fn print_version() {
    println!("MethodRay {}", env!("CARGO_PKG_VERSION"));
}
