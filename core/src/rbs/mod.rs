//! RBS type loading and conversion

// Converter is always available (no Ruby FFI dependency)
pub mod converter;
pub use converter::RbsTypeConverter;

// These require Ruby FFI for RBS loading
#[cfg(feature = "ruby-ffi")]
pub mod error;
#[cfg(feature = "ruby-ffi")]
pub mod loader;

#[cfg(feature = "ruby-ffi")]
pub use error::RbsError;
#[cfg(feature = "ruby-ffi")]
pub use loader::{register_rbs_methods, RbsLoader, RbsMethodInfo};

#[cfg(test)]
mod tests {
    #[test]
    fn test_embedded_method_loader_contains_expected_class() {
        let ruby_code = include_str!("method_loader.rb");
        assert!(
            ruby_code.contains("class MethodLoader"),
            "Embedded Ruby code should contain MethodLoader class definition"
        );
        assert!(
            ruby_code.contains("def load_methods"),
            "Embedded Ruby code should contain load_methods method"
        );
    }

    #[test]
    fn test_embedded_method_loader_has_no_absolute_paths() {
        let ruby_code = include_str!("method_loader.rb");
        let forbidden_patterns = ["/home/runner/", "/Users/", "/tmp/build/"];
        for pattern in &forbidden_patterns {
            assert!(
                !ruby_code.contains(pattern),
                "Embedded Ruby code should not contain absolute path: {}",
                pattern
            );
        }
    }
}
