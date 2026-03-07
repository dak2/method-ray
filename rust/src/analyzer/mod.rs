mod attributes;
mod blocks;
mod calls;
mod conditionals;
mod definitions;
mod dispatch;
mod install;
mod literals;
mod loops;
mod operators;
mod parameters;
mod parentheses;
mod returns;
mod variables;

pub use install::AstInstaller;

/// Convert ruby-prism identifier bytes to a String (lossy).
///
/// ruby-prism returns identifiers (method names, variable names, constant names,
/// parameter names) as `&[u8]`. This helper provides a single conversion point
/// used throughout the analyzer.
///
/// Note: Uses `from_utf8_lossy` — invalid UTF-8 bytes are replaced with U+FFFD.
/// ruby-prism identifiers are expected to be valid UTF-8, so this should not
/// occur in practice. Do NOT use this function for arbitrary byte data such as
/// string literal contents.
pub(crate) fn bytes_to_name(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

#[cfg(test)]
mod tests {
    use super::bytes_to_name;

    #[test]
    fn test_bytes_to_name_valid_utf8() {
        assert_eq!(bytes_to_name(b"hello"), "hello");
    }

    #[test]
    fn test_bytes_to_name_invalid_utf8_replaced() {
        assert_eq!(bytes_to_name(b"hello\xff"), "hello\u{FFFD}");
    }
}
