//! Node Dispatch - Dispatch AST nodes to appropriate handlers
//!
//! This module handles the pattern matching of Ruby AST nodes
//! and dispatches them to specialized handlers.

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{BlockParameterTypeBox, ChangeSet, VertexId};
use crate::source_map::SourceLocation;
use ruby_prism::Node;

use super::calls::install_method_call;
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
        /// Optional block attached to the method call
        block: Option<Node<'a>>,
        /// Arguments to the method call
        arguments: Vec<Node<'a>>,
    },
}

/// First pass: check if node can be handled immediately without child processing
///
/// Note: Literals (including Array) are handled in install.rs via install_literal
/// because Array literals need child processing for element type inference.
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

    // Method call: x.upcase or x.each { |i| ... }
    if let Some(call_node) = node.as_call_node() {
        if let Some(receiver) = call_node.receiver() {
            let method_name = String::from_utf8_lossy(call_node.name().as_slice()).to_string();
            // Use call_operator_loc (.) for error position, fallback to node location
            let prism_location = call_node
                .call_operator_loc()
                .unwrap_or_else(|| node.location());
            let location =
                SourceLocation::from_prism_location_with_source(&prism_location, source);

            // Get block if present (e.g., `x.each { |i| ... }`)
            let block = call_node.block();

            // Get arguments if present (e.g., `x.foo(1, 2)`)
            let arguments: Vec<Node<'a>> = call_node
                .arguments()
                .map(|args| args.arguments().iter().collect())
                .unwrap_or_default();

            return Some(NeedsChildKind::MethodCall {
                receiver,
                method_name,
                location,
                block,
                arguments,
            });
        }
    }

    None
}

/// Process a node that needs child processing
///
/// This function handles the second phase of two-phase dispatch:
/// 1. `dispatch_needs_child` identifies the node kind and extracts data
/// 2. `process_needs_child` processes child nodes and completes the operation
pub(crate) fn process_needs_child(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    kind: NeedsChildKind,
) -> Option<VertexId> {
    match kind {
        NeedsChildKind::IvarWrite { ivar_name, value } => {
            let value_vtx = super::install::install_node(genv, lenv, changes, source, &value)?;
            Some(finish_ivar_write(genv, ivar_name, value_vtx))
        }
        NeedsChildKind::LocalVarWrite { var_name, value } => {
            let value_vtx = super::install::install_node(genv, lenv, changes, source, &value)?;
            Some(finish_local_var_write(genv, lenv, changes, var_name, value_vtx))
        }
        NeedsChildKind::MethodCall {
            receiver,
            method_name,
            location,
            block,
            arguments,
        } => {
            let recv_vtx = super::install::install_node(genv, lenv, changes, source, &receiver)?;

            // Process arguments if present
            let arg_vtxs: Vec<VertexId> = arguments
                .iter()
                .filter_map(|arg| {
                    super::install::install_node(genv, lenv, changes, source, arg)
                })
                .collect();

            // Handle block if present (e.g., `x.each { |i| ... }`)
            if let Some(block_node) = block {
                if let Some(block) = block_node.as_block_node() {
                    let param_vtxs = super::blocks::process_block_node_with_params(
                        genv, lenv, changes, source, &block,
                    );

                    if !param_vtxs.is_empty() {
                        let box_id = genv.alloc_box_id();
                        let block_box = BlockParameterTypeBox::new(
                            box_id,
                            recv_vtx,
                            method_name.clone(),
                            param_vtxs,
                        );
                        genv.register_box(box_id, Box::new(block_box));
                    }
                }
            }

            Some(finish_method_call(
                genv, recv_vtx, method_name, arg_vtxs, location,
            ))
        }
    }
}

/// Finish instance variable write after child is processed
fn finish_ivar_write(genv: &mut GlobalEnv, ivar_name: String, value_vtx: VertexId) -> VertexId {
    install_ivar_write(genv, ivar_name, value_vtx)
}

/// Finish local variable write after child is processed
fn finish_local_var_write(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    var_name: String,
    value_vtx: VertexId,
) -> VertexId {
    install_local_var_write(genv, lenv, changes, var_name, value_vtx)
}

/// Finish method call after receiver is processed
fn finish_method_call(
    genv: &mut GlobalEnv,
    recv_vtx: VertexId,
    method_name: String,
    arg_vtxs: Vec<VertexId>,
    location: SourceLocation,
) -> VertexId {
    install_method_call(genv, recv_vtx, method_name, arg_vtxs, Some(location))
}
