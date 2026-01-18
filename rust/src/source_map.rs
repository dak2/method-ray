/// Convert byte offset to (line, column) - 1-indexed
fn offset_to_line_column(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut column = 1;
    let mut current_offset = 0;

    for ch in source.chars() {
        if current_offset >= offset {
            break;
        }

        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }

        current_offset += ch.len_utf8();
    }

    (line, column)
}

/// Source code location information
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub length: usize,
}

#[allow(dead_code)]
impl SourceLocation {
    pub fn new(line: usize, column: usize, length: usize) -> Self {
        Self {
            line,
            column,
            length,
        }
    }

    /// Create from ruby-prism Location and source code
    /// Calculates line/column from byte offset
    pub fn from_prism_location_with_source(
        location: &ruby_prism::Location,
        source: &str,
    ) -> Self {
        let start_offset = location.start_offset();
        let length = location.end_offset() - start_offset;

        // Calculate line and column from byte offset
        let (line, column) = offset_to_line_column(source, start_offset);

        Self {
            line,
            column,
            length,
        }
    }

    /// Create from ruby-prism Location (without source - uses approximation)
    pub fn from_prism_location(location: &ruby_prism::Location) -> Self {
        let start_offset = location.start_offset();
        let length = location.end_offset() - start_offset;

        // Without source, we can't calculate exact line/column
        // Use offset as column for now
        Self {
            line: 1, // Placeholder
            column: start_offset + 1,
            length,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_location_creation() {
        let loc = SourceLocation::new(10, 5, 6);
        assert_eq!(loc.line, 10);
        assert_eq!(loc.column, 5);
        assert_eq!(loc.length, 6);
    }

    #[test]
    fn test_offset_to_line_column() {
        let source = "x = 1\ny = x.upcase";
        // "x = 1\n" is 6 bytes (0-5)
        // "y = x.upcase" starts at offset 6
        // "y = x." is 6 bytes, so ".upcase" starts at offset 12

        // Test offset 0 (start of line 1)
        assert_eq!(offset_to_line_column(source, 0), (1, 1));

        // Test offset 6 (start of line 2, after newline)
        assert_eq!(offset_to_line_column(source, 6), (2, 1));

        // Test offset 10 (the 'x' in 'x.upcase')
        assert_eq!(offset_to_line_column(source, 10), (2, 5));
    }
}
