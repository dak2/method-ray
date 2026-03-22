use anyhow::{Context, Result};
use bumpalo::Bump;
use ruby_prism::{parse, ParseResult};
use std::fs;
use std::path::Path;

/// Parse session - manages source bytes for multiple files using arena allocation
///
/// Uses an arena allocator to efficiently manage source bytes during parsing.
/// When the session is dropped, all memory is released at once.
pub struct ParseSession {
    arena: Bump,
}

impl ParseSession {
    pub fn new() -> Self {
        Self { arena: Bump::new() }
    }

    /// Create with pre-allocated capacity (recommended for batch file processing)
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            arena: Bump::with_capacity(capacity),
        }
    }

    /// Allocate source in arena and parse
    pub fn parse_source<'a>(&'a self, source: &str, file_name: &str) -> Result<ParseResult<'a>> {
        // Copy bytes to arena
        let source_bytes = self.arena.alloc_slice_copy(source.as_bytes());
        let parse_result = parse(source_bytes);

        // Check for parse errors
        let error_messages: Vec<String> = parse_result
            .errors()
            .map(|e| {
                format!(
                    "Parse error at offset {}: {}",
                    e.location().start_offset(),
                    e.message()
                )
            })
            .collect();

        if !error_messages.is_empty() {
            anyhow::bail!(
                "Failed to parse Ruby source in {}:\n{}",
                file_name,
                error_messages.join("\n")
            );
        }

        Ok(parse_result)
    }

    /// Read file and parse
    pub fn parse_file<'a>(&'a self, file_path: &Path) -> Result<ParseResult<'a>> {
        let source = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

        self.parse_source(&source, &file_path.to_string_lossy())
    }

    /// Get allocated memory size (for debugging)
    pub fn allocated_bytes(&self) -> usize {
        self.arena.allocated_bytes()
    }

    /// Reset arena (for memory control during batch file processing)
    pub fn reset(&mut self) {
        self.arena.reset();
    }
}

impl Default for ParseSession {
    fn default() -> Self {
        Self::new()
    }
}
