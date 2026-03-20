//! Return statement handling
//!
//! Processes `return expr` by connecting the expression's vertex
//! to the enclosing method's merge vertex.

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};

use super::install::install_node;

/// Process ReturnNode: connect return value to method's merge vertex
pub(crate) fn process_return_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    return_node: &ruby_prism::ReturnNode,
) -> Option<VertexId> {
    // Process return value (first argument only; multi-value return not yet supported)
    let value_vtx = if let Some(arguments) = return_node.arguments() {
        arguments
            .arguments()
            .iter()
            .next()
            .and_then(|arg| install_node(genv, lenv, changes, source, &arg))
    } else {
        // `return` without value → nil
        Some(genv.new_source(crate::types::Type::Nil))
    };

    // Connect return value to method's merge vertex
    if let Some(vtx) = value_vtx {
        if let Some(merge_vtx) = genv.scope_manager.current_method_return_vertex() {
            genv.add_edge(vtx, merge_vtx);
        }
    }

    None
}
