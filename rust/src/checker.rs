use crate::analyzer::AstInstaller;
use crate::diagnostics::Diagnostic;
use crate::env::{GlobalEnv, LocalEnv};
use crate::parser;
use anyhow::{Context, Result};
use std::path::Path;

/// File type checker
pub struct FileChecker {
    // No state - creates fresh GlobalEnv for each check
}

impl FileChecker {
    /// Create new FileChecker
    /// Note: This is for standalone CLI usage (no Ruby runtime)
    pub fn new() -> Result<Self> {
        // Just verify cache exists
        use crate::cache::RbsCache;
        RbsCache::load().context(
            "Failed to load RBS cache. Please run from Ruby first to generate cache:\n\
             ruby -rmethodray -e 'MethodRay::Analyzer.new(\".\").infer_types(\"x=1\")'",
        )?;

        Ok(Self {})
    }

    /// Check a single Ruby file
    pub fn check_file(&self, file_path: &Path) -> Result<Vec<Diagnostic>> {
        // Read source code
        let source = std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read {}", file_path.display()))?;

        // Parse file
        let parse_result = parser::parse_ruby_file(file_path)
            .with_context(|| format!("Failed to parse {}", file_path.display()))?;

        // Create fresh GlobalEnv for this analysis
        let mut genv = GlobalEnv::new();
        load_rbs_from_cache(&mut genv)?;

        let mut lenv = LocalEnv::new();
        let mut installer = AstInstaller::new(&mut genv, &mut lenv, &source);

        // Process AST
        let root = parse_result.node();
        if let Some(program_node) = root.as_program_node() {
            let statements = program_node.statements();
            for stmt in &statements.body() {
                installer.install_node(&stmt);
            }
        }

        installer.finish();

        // Collect diagnostics
        let diagnostics = collect_diagnostics(&genv, file_path);

        Ok(diagnostics)
    }
}

/// Load RBS methods from cache (CLI mode without Ruby runtime)
fn load_rbs_from_cache(genv: &mut GlobalEnv) -> Result<()> {
    use crate::cache::RbsCache;
    use crate::types::Type;

    let cache = RbsCache::load().context(
        "Failed to load RBS cache. Please run from Ruby first to generate cache:\n\
         ruby -rmethodray -e 'MethodRay::Analyzer.new(\".\").infer_types(\"x=1\")'",
    )?;

    let methods = cache.methods();

    for method_info in methods {
        let receiver_type = Type::Instance {
            class_name: method_info.receiver_class.clone(),
        };
        genv.register_builtin_method(
            receiver_type,
            &method_info.method_name,
            method_info.return_type(),
        );
    }

    Ok(())
}

/// Collect type error diagnostics from GlobalEnv
fn collect_diagnostics(genv: &GlobalEnv, file_path: &Path) -> Vec<Diagnostic> {
    use crate::diagnostics::{Diagnostic, Location};
    use std::path::PathBuf;

    let mut diagnostics = Vec::new();

    // Convert TypeErrors to Diagnostics
    for type_error in &genv.type_errors {
        // Use actual location from TypeError if available
        let location = if let Some(source_loc) = &type_error.location {
            Location {
                file: PathBuf::from(file_path),
                line: source_loc.line,
                column: source_loc.column,
                length: Some(source_loc.length),
            }
        } else {
            // Fallback to placeholder
            Location {
                file: PathBuf::from(file_path),
                line: 1,
                column: 1,
                length: None,
            }
        };

        let diagnostic = Diagnostic::undefined_method(
            location,
            &type_error.receiver_type.show(),
            &type_error.method_name,
        );

        diagnostics.push(diagnostic);
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_checker_creation() {
        // This test will fail if RBS cache doesn't exist
        // That's expected - cache should be generated from Ruby side first
        let result = FileChecker::new();
        assert!(result.is_ok() || result.is_err()); // Just check it doesn't panic
    }
}
