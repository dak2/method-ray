//! Type error definitions for diagnostic reporting

use crate::graph::VertexId;
use crate::source_map::SourceLocation;
use crate::types::Type;

/// Type error information for diagnostic reporting
#[derive(Debug, Clone)]
pub struct TypeError {
    pub receiver_type: Type,
    pub method_name: String,
    pub vertex_id: VertexId,
    pub location: Option<SourceLocation>,
}

impl TypeError {
    /// Create a new type error
    pub fn new(
        receiver_type: Type,
        method_name: String,
        vertex_id: VertexId,
        location: Option<SourceLocation>,
    ) -> Self {
        Self {
            receiver_type,
            method_name,
            vertex_id,
            location,
        }
    }
}
