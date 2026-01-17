use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use crate::diagnostics::{Diagnostic as MethodRayDiagnostic, DiagnosticLevel};

/// Extract method name length from error message
/// Supports messages like:
/// - "undefined method `downcase` for Integer"
/// - "method `upcase` is defined for ..."
fn extract_method_name_length(message: &str) -> Option<u32> {
    // Pattern: `method_name`
    if let Some(start) = message.find('`') {
        if let Some(end) = message[start + 1..].find('`') {
            let method_name = &message[start + 1..start + 1 + end];
            return Some(method_name.len() as u32);
        }
    }
    None
}

/// Convert MethodRay Diagnostic to LSP Diagnostic
pub fn to_lsp_diagnostic(diag: &MethodRayDiagnostic) -> Diagnostic {
    let severity = match diag.level {
        DiagnosticLevel::Error => DiagnosticSeverity::ERROR,
        DiagnosticLevel::Warning => DiagnosticSeverity::WARNING,
    };

    let start_line = if diag.location.line > 0 {
        (diag.location.line - 1) as u32
    } else {
        0
    };

    let start_char = if diag.location.column > 0 {
        (diag.location.column - 1) as u32
    } else {
        0
    };

    // Use actual source length if available, otherwise extract from message
    let highlight_length = diag.location.length
        .map(|len| len as u32)
        .or_else(|| extract_method_name_length(&diag.message))
        .unwrap_or(5);
    let end_char = start_char + highlight_length;

    Diagnostic {
        range: Range {
            start: Position {
                line: start_line,
                character: start_char,
            },
            end: Position {
                line: start_line,
                character: end_char,
            },
        },
        severity: Some(severity),
        code: None,
        code_description: None,
        source: Some("methodray".to_string()),
        message: diag.message.clone(),
        related_information: None,
        tags: None,
        data: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Location;

    #[test]
    fn test_to_lsp_diagnostic() {
        use std::path::PathBuf;

        let methodray_diag = MethodRayDiagnostic {
            level: DiagnosticLevel::Error,
            location: Location {
                file: PathBuf::from("test.rb"),
                line: 5,
                column: 10,
                length: Some(6), // "upcase".len()
            },
            message: "undefined method `upcase` for Integer".to_string(),
            code: None,
        };

        let lsp_diag = to_lsp_diagnostic(&methodray_diag);

        assert_eq!(lsp_diag.range.start.line, 4); // 0-indexed
        assert_eq!(lsp_diag.range.start.character, 9); // 0-indexed
        assert_eq!(lsp_diag.range.end.character, 15); // start(9) + length(6)
        assert_eq!(lsp_diag.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(lsp_diag.message, "undefined method `upcase` for Integer");
    }

    #[test]
    fn test_extract_method_name_length() {
        assert_eq!(
            extract_method_name_length("undefined method `downcase` for Integer"),
            Some(8)
        );
        assert_eq!(
            extract_method_name_length("method `upcase` is defined for String"),
            Some(6)
        );
        assert_eq!(
            extract_method_name_length("no method name here"),
            None
        );
    }

    #[test]
    fn test_highlight_length_for_downcase() {
        use std::path::PathBuf;

        let methodray_diag = MethodRayDiagnostic {
            level: DiagnosticLevel::Error,
            location: Location {
                file: PathBuf::from("test.rb"),
                line: 2,
                column: 5,
                length: Some(8), // "downcase".len()
            },
            message: "undefined method `downcase` for Integer".to_string(),
            code: None,
        };

        let lsp_diag = to_lsp_diagnostic(&methodray_diag);

        assert_eq!(lsp_diag.range.start.character, 4); // column 5 -> 0-indexed = 4
        assert_eq!(lsp_diag.range.end.character, 12); // start(4) + length(8)
    }
}
