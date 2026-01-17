use super::diagnostic::Diagnostic;
use std::fs;
use std::path::Path;

/// Format diagnostics in LSP-compatible format
///
/// Example output:
/// ```
/// app/models/user.rb:10:5: error: undefined method `upcase` for Integer
/// ```
pub fn format_diagnostics(diagnostics: &[Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(|diag| {
            format!(
                "{}:{}:{}: {}: {}",
                diag.location.file.display(),
                diag.location.line,
                diag.location.column,
                diag.level.as_str(),
                diag.message
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Format diagnostics with code snippet
///
/// Example output:
/// ```
/// app/models/user.rb:10:5: error: undefined method `upcase` for Integer
///    x.upcase
///      ^^^^^^
/// ```
pub fn format_diagnostics_with_source(diagnostics: &[Diagnostic], source_code: &str) -> String {
    let lines: Vec<&str> = source_code.lines().collect();

    diagnostics
        .iter()
        .map(|diag| {
            let mut output = format!(
                "{}:{}:{}: {}: {}",
                diag.location.file.display(),
                diag.location.line,
                diag.location.column,
                diag.level.as_str(),
                diag.message
            );

            // Add code snippet
            if diag.location.line > 0 && diag.location.line <= lines.len() {
                let line_index = diag.location.line - 1; // Convert to 0-indexed
                let source_line = lines[line_index];

                output.push('\n');
                output.push_str("   ");
                output.push_str(source_line);
                output.push('\n');

                // Add caret indicator
                let column = diag.location.column.saturating_sub(1); // Convert to 0-indexed
                output.push_str("   ");
                output.push_str(&" ".repeat(column));
                output.push('^');
            }

            output
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Read source file and format diagnostics with code snippet
pub fn format_diagnostics_with_file(diagnostics: &[Diagnostic], file_path: &Path) -> String {
    match fs::read_to_string(file_path) {
        Ok(source) => format_diagnostics_with_source(diagnostics, &source),
        Err(_) => format_diagnostics(diagnostics), // Fallback to simple format
    }
}

/// Format diagnostics with detailed output (for debugging)
pub fn format_diagnostics_detailed(diagnostics: &[Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(|diag| {
            let mut output = format!(
                "{}:{}:{}: {}: {}",
                diag.location.file.display(),
                diag.location.line,
                diag.location.column,
                diag.level.as_str(),
                diag.message
            );

            if let Some(code) = &diag.code {
                output.push_str(&format!(" [{}]", code));
            }

            output
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::diagnostic::{Diagnostic, Location};
    use std::path::PathBuf;

    #[test]
    fn test_format_diagnostics() {
        let diagnostics = vec![
            Diagnostic::undefined_method(
                Location {
                    file: PathBuf::from("test.rb"),
                    line: 10,
                    column: 5,
                    length: None,
                },
                "Integer",
                "upcase",
            ),
            Diagnostic::warning(
                Location {
                    file: PathBuf::from("test.rb"),
                    line: 15,
                    column: 3,
                    length: None,
                },
                "unused variable".to_string(),
            ),
        ];

        let output = format_diagnostics(&diagnostics);
        assert!(output.contains("test.rb:10:5: error:"));
        assert!(output.contains("test.rb:15:3: warning:"));
    }
}
