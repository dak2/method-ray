//! Method Call Handlers - Processing Ruby method calls
//!
//! This module is responsible for:
//! - Creating MethodCallBox for method invocations (x.upcase)
//! - Managing return value vertices
//! - Attaching source location for error reporting

use std::collections::HashMap;

use crate::env::GlobalEnv;
use crate::graph::{MethodCallBox, VertexId};
use crate::source_map::SourceLocation;

/// Install method call and return the return value's VertexId
pub(crate) fn install_method_call(
    genv: &mut GlobalEnv,
    recv_vtx: VertexId,
    method_name: String,
    arg_vtxs: Vec<VertexId>,
    kwarg_vtxs: Option<HashMap<String, VertexId>>,
    location: Option<SourceLocation>,
    safe_navigation: bool,
) -> VertexId {
    // Create Vertex for return value
    let ret_vtx = genv.new_vertex();

    // Create MethodCallBox with location and argument vertices
    let box_id = genv.alloc_box_id();
    let call_box =
        MethodCallBox::new(box_id, recv_vtx, method_name, ret_vtx, arg_vtxs, kwarg_vtxs, location, safe_navigation);
    genv.register_box(box_id, Box::new(call_box));

    ret_vtx
}
