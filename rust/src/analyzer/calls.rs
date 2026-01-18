//! Method Call Handlers - Processing Ruby method calls
//!
//! This module is responsible for:
//! - Creating MethodCallBox for method invocations (x.upcase)
//! - Managing return value vertices
//! - Attaching source location for error reporting

use crate::env::GlobalEnv;
use crate::graph::{MethodCallBox, VertexId};
use crate::source_map::SourceLocation;

/// Install method call and return the return value's VertexId
pub fn install_method_call(
    genv: &mut GlobalEnv,
    recv_vtx: VertexId,
    method_name: String,
    location: Option<SourceLocation>,
) -> VertexId {
    // Create Vertex for return value
    let ret_vtx = genv.new_vertex();

    // Create MethodCallBox with location
    let box_id = genv.alloc_box_id();
    let call_box = MethodCallBox::new(box_id, recv_vtx, method_name, ret_vtx, location);
    genv.register_box(box_id, Box::new(call_box));

    ret_vtx
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Type;

    #[test]
    fn test_install_method_call_creates_vertex() {
        let mut genv = GlobalEnv::new();

        let recv_vtx = genv.new_source(Type::string());
        let ret_vtx = install_method_call(&mut genv, recv_vtx, "upcase".to_string(), None);

        // Return vertex should exist
        assert!(genv.get_vertex(ret_vtx).is_some());
    }

    #[test]
    fn test_install_method_call_adds_box() {
        let mut genv = GlobalEnv::new();

        let recv_vtx = genv.new_source(Type::string());
        let _ret_vtx = install_method_call(&mut genv, recv_vtx, "upcase".to_string(), None);

        // Box should be added
        assert_eq!(genv.box_count(), 1);
    }
}
