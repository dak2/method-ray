use std::path::PathBuf;

/// Diagnostic severity level (LSP compatible)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum DiagnosticLevel {
    Error,
    Warning,
}

impl DiagnosticLevel {
    pub fn as_str(&self) -> &str {
        match self {
            DiagnosticLevel::Error => "error",
            DiagnosticLevel::Warning => "warning",
        }
    }
}

/// Source code location
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub length: Option<usize>, // Character length of the error span
}

/// Type checking diagnostic
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Diagnostic {
    pub location: Location,
    pub level: DiagnosticLevel,
    pub message: String,
    pub code: Option<String>, // e.g., "E001"
}

#[allow(dead_code)]
impl Diagnostic {
    /// Create an error diagnostic
    pub fn error(location: Location, message: String) -> Self {
        Self {
            location,
            level: DiagnosticLevel::Error,
            message,
            code: None,
        }
    }

    /// Create a warning diagnostic (internal use only)
    fn warning(location: Location, message: String) -> Self {
        Self {
            location,
            level: DiagnosticLevel::Warning,
            message,
            code: None,
        }
    }

    /// Create undefined method error
    pub fn undefined_method(
        location: Location,
        receiver_type: &str,
        method_name: &str,
    ) -> Self {
        Self::error(
            location,
            format!(
                "undefined method `{}` for {}",
                method_name, receiver_type
            ),
        )
    }

    /// Create Union type partial error (warning)
    pub fn union_partial_error(
        location: Location,
        valid_types: Vec<String>,
        invalid_types: Vec<String>,
        method_name: &str,
    ) -> Self {
        let message = format!(
            "method `{}` is defined for {} but not for {}",
            method_name,
            valid_types.join(", "),
            invalid_types.join(", ")
        );
        Self::warning(location, message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_creation() {
        let loc = Location {
            file: PathBuf::from("test.rb"),
            line: 10,
            column: 5,
            length: None,
        };

        let diag = Diagnostic::undefined_method(loc.clone(), "Integer", "upcase");
        assert_eq!(diag.level, DiagnosticLevel::Error);
        assert_eq!(diag.message, "undefined method `upcase` for Integer");
    }

    #[test]
    fn test_union_partial_error() {
        let loc = Location {
            file: PathBuf::from("test.rb"),
            line: 15,
            column: 3,
            length: None,
        };

        let diag = Diagnostic::union_partial_error(
            loc,
            vec!["String".to_string()],
            vec!["Integer".to_string()],
            "upcase",
        );

        assert_eq!(diag.level, DiagnosticLevel::Warning);
        assert!(diag
            .message
            .contains("method `upcase` is defined for String but not for Integer"));
    }
}
