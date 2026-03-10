//! Method-Ray Core - Static type checker for Ruby
//!
//! This crate provides the core type inference engine.

pub mod analyzer;
pub mod cache;
pub mod diagnostics;
pub mod env;
pub mod graph;
pub mod parser;
pub mod source_map;
pub mod types;

// rbs module is always available (converter has no Ruby FFI dependency)
// but loader and error require ruby-ffi feature
pub mod rbs;

#[cfg(any(feature = "cli", feature = "lsp"))]
pub mod checker;

#[cfg(feature = "cli")]
pub mod cli;

#[cfg(feature = "lsp")]
pub mod lsp;
