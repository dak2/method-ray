//! Type error definitions for diagnostic reporting

use crate::source_map::SourceLocation;
use crate::types::Type;

/// Type error information for diagnostic reporting
#[derive(Debug, Clone)]
pub struct TypeError {
    pub receiver_type: Type,
    pub method_name: String,
    pub location: Option<SourceLocation>,
}

impl TypeError {
    /// Create a new type error
    pub fn new(receiver_type: Type, method_name: String, location: Option<SourceLocation>) -> Self {
        Self {
            receiver_type,
            method_name,
            location,
        }
    }
}
