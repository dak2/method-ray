//! Node Dispatch - Dispatch AST nodes to appropriate handlers
//!
//! This module handles the pattern matching of Ruby AST nodes
//! and dispatches them to specialized handlers.

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};
use crate::source_map::SourceLocation;
use ruby_prism::Node;

use super::calls::install_method_call;
use super::literals::install_literal;
use super::variables::{
    install_ivar_read, install_ivar_write, install_local_var_read, install_local_var_write,
    install_self,
};

/// Result of dispatching a simple node (no child processing needed)
pub enum DispatchResult {
    /// Node produced a vertex
    Vertex(VertexId),
    /// Node was not handled
    NotHandled,
}

/// Kind of child processing needed
pub enum NeedsChildKind<'a> {
    /// Instance variable write: need to process value, then call finish_ivar_write
    IvarWrite { ivar_name: String, value: Node<'a> },
    /// Local variable write: need to process value, then call finish_local_var_write
    LocalVarWrite { var_name: String, value: Node<'a> },
    /// Method call: need to process receiver, then call finish_method_call
    MethodCall {
        receiver: Node<'a>,
        method_name: String,
        location: SourceLocation,
    },
}

/// First pass: check if node can be handled immediately without child processing
pub fn dispatch_simple(genv: &mut GlobalEnv, lenv: &mut LocalEnv, node: &Node) -> DispatchResult {
    // Instance variable read: @name
    if let Some(ivar_read) = node.as_instance_variable_read_node() {
        let ivar_name = String::from_utf8_lossy(ivar_read.name().as_slice()).to_string();
        return match install_ivar_read(genv, &ivar_name) {
            Some(vtx) => DispatchResult::Vertex(vtx),
            None => DispatchResult::NotHandled,
        };
    }

    // self
    if node.as_self_node().is_some() {
        return DispatchResult::Vertex(install_self(genv));
    }

    // Local variable read: x
    if let Some(read_node) = node.as_local_variable_read_node() {
        let var_name = String::from_utf8_lossy(read_node.name().as_slice()).to_string();
        return match install_local_var_read(lenv, &var_name) {
            Some(vtx) => DispatchResult::Vertex(vtx),
            None => DispatchResult::NotHandled,
        };
    }

    // Literals (String, Integer, Array, Hash, nil, true, false, Symbol)
    if let Some(vtx) = install_literal(genv, node) {
        return DispatchResult::Vertex(vtx);
    }

    DispatchResult::NotHandled
}

/// Check if node needs child processing
pub fn dispatch_needs_child<'a>(node: &Node<'a>, source: &str) -> Option<NeedsChildKind<'a>> {
    // Instance variable write: @name = value
    if let Some(ivar_write) = node.as_instance_variable_write_node() {
        let ivar_name = String::from_utf8_lossy(ivar_write.name().as_slice()).to_string();
        return Some(NeedsChildKind::IvarWrite {
            ivar_name,
            value: ivar_write.value(),
        });
    }

    // Local variable write: x = value
    if let Some(write_node) = node.as_local_variable_write_node() {
        let var_name = String::from_utf8_lossy(write_node.name().as_slice()).to_string();
        return Some(NeedsChildKind::LocalVarWrite {
            var_name,
            value: write_node.value(),
        });
    }

    // Method call: x.upcase
    if let Some(call_node) = node.as_call_node() {
        if let Some(receiver) = call_node.receiver() {
            let method_name = String::from_utf8_lossy(call_node.name().as_slice()).to_string();
            let location =
                SourceLocation::from_prism_location_with_source(&node.location(), source);
            return Some(NeedsChildKind::MethodCall {
                receiver,
                method_name,
                location,
            });
        }
    }

    None
}

/// Finish instance variable write after child is processed
pub fn finish_ivar_write(genv: &mut GlobalEnv, ivar_name: String, value_vtx: VertexId) -> VertexId {
    install_ivar_write(genv, ivar_name, value_vtx)
}

/// Finish local variable write after child is processed
pub fn finish_local_var_write(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    var_name: String,
    value_vtx: VertexId,
) -> VertexId {
    install_local_var_write(genv, lenv, changes, var_name, value_vtx)
}

/// Finish method call after receiver is processed
pub fn finish_method_call(
    genv: &mut GlobalEnv,
    recv_vtx: VertexId,
    method_name: String,
    location: SourceLocation,
) -> VertexId {
    install_method_call(genv, recv_vtx, method_name, Some(location))
}
