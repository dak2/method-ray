//! Node Dispatch - Dispatch AST nodes to appropriate handlers
//!
//! This module handles the pattern matching of Ruby AST nodes
//! and dispatches them to specialized handlers.

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{BlockParameterTypeBox, ChangeSet, VertexId};
use crate::source_map::SourceLocation;
use crate::types::Type;
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
    /// Implicit self method call: method call without explicit receiver (implicit self)
    ImplicitSelfCall {
        method_name: String,
        location: SourceLocation,
        block: Option<Node<'a>>,
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

    // Method call: x.upcase, x.each { |i| ... }, or name (implicit self)
    if let Some(call_node) = node.as_call_node() {
        let method_name = String::from_utf8_lossy(call_node.name().as_slice()).to_string();
        let block = call_node.block();
        let arguments: Vec<Node<'a>> = call_node
            .arguments()
            .map(|args| args.arguments().iter().collect())
            .unwrap_or_default();

        if let Some(receiver) = call_node.receiver() {
            // Explicit receiver: x.upcase, x.each { |i| ... }
            let prism_location = call_node
                .call_operator_loc()
                .unwrap_or_else(|| node.location());
            let location =
                SourceLocation::from_prism_location_with_source(&prism_location, source);

            return Some(NeedsChildKind::MethodCall {
                receiver,
                method_name,
                location,
                block,
                arguments,
            });
        } else {
            // No receiver: implicit self method call (e.g., `name`, `puts "hello"`)

            // TODO: attr_* methods will be handled in next phase (skip for now)
            const ATTR_METHODS: &[&str] = &["attr_reader", "attr_writer", "attr_accessor"];
            if ATTR_METHODS.contains(&method_name.as_str()) {
                return None;
            }

            let prism_location = call_node
                .message_loc()
                .unwrap_or_else(|| node.location());
            let location =
                SourceLocation::from_prism_location_with_source(&prism_location, source);

            return Some(NeedsChildKind::ImplicitSelfCall {
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
            process_method_call_common(
                genv, lenv, changes, source, recv_vtx, method_name, location, block, arguments,
            )
        }
        NeedsChildKind::ImplicitSelfCall {
            method_name,
            location,
            block,
            arguments,
        } => {
            // Use the same naming as method registration in definitions.rs
            let recv_type_name = genv
                .scope_manager
                .current_class_name()
                .or_else(|| genv.scope_manager.current_module_name());
            let recv_vtx = if let Some(name) = recv_type_name {
                genv.new_source(Type::instance(&name))
            } else {
                genv.new_source(Type::instance("Object"))
            };
            process_method_call_common(
                genv, lenv, changes, source, recv_vtx, method_name, location, block, arguments,
            )
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

/// MethodCall / ImplicitSelfCall common processing:
/// Handles argument processing, block processing, and MethodCallBox creation after recv_vtx is obtained
fn process_method_call_common<'a>(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    recv_vtx: VertexId,
    method_name: String,
    location: SourceLocation,
    block: Option<Node<'a>>,
    arguments: Vec<Node<'a>>,
) -> Option<VertexId> {
    let arg_vtxs: Vec<VertexId> = arguments
        .iter()
        .filter_map(|arg| super::install::install_node(genv, lenv, changes, source, arg))
        .collect();

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::install::AstInstaller;
    use crate::parser::ParseSession;

    /// Helper: parse Ruby source, process with AstInstaller, and return GlobalEnv
    fn analyze(source: &str) -> GlobalEnv {
        let session = ParseSession::new();
        let parse_result = session.parse_source(source, "test.rb").unwrap();
        let root = parse_result.node();
        let program = root.as_program_node().unwrap();

        let mut genv = GlobalEnv::new();
        let mut lenv = LocalEnv::new();

        let mut installer = AstInstaller::new(&mut genv, &mut lenv, source);
        for stmt in &program.statements().body() {
            installer.install_node(&stmt);
        }
        installer.finish();

        genv
    }

    /// Helper: get the type string for a vertex ID (checks both Vertex and Source)
    fn get_type_show(genv: &GlobalEnv, vtx: VertexId) -> String {
        if let Some(vertex) = genv.get_vertex(vtx) {
            vertex.show()
        } else if let Some(source) = genv.get_source(vtx) {
            source.ty.show()
        } else {
            panic!("vertex {:?} not found as either Vertex or Source", vtx);
        }
    }

    // Test 1: Receiverless method call type resolution
    #[test]
    fn test_implicit_self_call_type_resolution() {
        let source = r#"
class User
  def name
    "Alice"
  end

  def greet
    name
  end
end
"#;
        let genv = analyze(source);

        // User#greet should resolve to String via User#name
        let info = genv
            .resolve_method(&Type::instance("User"), "greet")
            .expect("User#greet should be registered");
        assert!(info.return_vertex.is_some());

        let ret_vtx = info.return_vertex.unwrap();
        assert_eq!(get_type_show(&genv, ret_vtx), "String");
    }

    // Test 2: Receiverless method call with arguments
    #[test]
    fn test_implicit_self_call_with_arguments() {
        let source = r#"
class Calculator
  def add(x, y)
    x
  end

  def compute
    add(1, 2)
  end
end
"#;
        let genv = analyze(source);

        // Calculator#compute should resolve via Calculator#add
        let info = genv
            .resolve_method(&Type::instance("Calculator"), "compute")
            .expect("Calculator#compute should be registered");
        assert!(info.return_vertex.is_some());

        let ret_vtx = info.return_vertex.unwrap();
        assert_eq!(get_type_show(&genv, ret_vtx), "Integer");
    }

    // Test 3: Receiverless call in nested class
    #[test]
    fn test_implicit_self_call_in_nested_class() {
        let source = r#"
module Api
  module V1
    class User
      def name
        "Alice"
      end

      def greet
        name
      end
    end
  end
end
"#;
        let genv = analyze(source);

        // Method registered with simple class name "User" (current behavior of definitions.rs)
        let info = genv
            .resolve_method(&Type::instance("User"), "greet")
            .expect("User#greet should be registered");
        assert!(info.return_vertex.is_some());

        let ret_vtx = info.return_vertex.unwrap();
        assert_eq!(get_type_show(&genv, ret_vtx), "String");
    }

    // Test 4: Receiverless call in module
    #[test]
    fn test_implicit_self_call_in_module() {
        let source = r#"
module Utils
  def self.format(value)
    value
  end

  def self.run
    format("test")
  end
end
"#;
        let genv = analyze(source);

        // Utils.run should be registered
        let info = genv
            .resolve_method(&Type::instance("Utils"), "run")
            .expect("Utils#run should be registered");
        assert!(info.return_vertex.is_some());
    }

    // Test 5: Receiverless call from within block
    #[test]
    fn test_implicit_self_call_from_block() {
        let source = r#"
class User
  def name
    "Alice"
  end

  def greet
    [1].each { name }
  end
end
"#;
        let genv = analyze(source);

        // User#name should be registered and resolve to String
        let name_info = genv
            .resolve_method(&Type::instance("User"), "name")
            .expect("User#name should be registered");
        assert!(name_info.return_vertex.is_some());
        assert_eq!(get_type_show(&genv, name_info.return_vertex.unwrap()), "String");

        // User#greet should also be registered (block contains implicit self call)
        let greet_info = genv
            .resolve_method(&Type::instance("User"), "greet")
            .expect("User#greet should be registered");
        assert!(greet_info.return_vertex.is_some());
    }

    // Test 6: Top-level receiverless call (Object receiver)
    #[test]
    fn test_implicit_self_call_at_top_level() {
        let source = r#"
def helper
  "result"
end

helper
"#;
        let genv = analyze(source);

        // Should not panic; top-level call uses Object as receiver type
        // NOTE: top-level def is not registered in method_registry yet,
        // so this will produce a type error (false positive).
        // The important thing is that it doesn't panic.
        // No panic is the real assertion - top-level call should be processed without error
        let _ = genv;
    }

    // Test 7: attr_* methods are skipped by dispatch_needs_child
    #[test]
    fn test_attr_reader_skipped() {
        let source = r#"
class User
  attr_reader :name

  def greet
    "hello"
  end
end
"#;
        let genv = analyze(source);

        // attr_reader should be skipped without panic, and other methods should work
        let info = genv
            .resolve_method(&Type::instance("User"), "greet")
            .expect("User#greet should be registered");
        assert!(info.return_vertex.is_some());

        let ret_vtx = info.return_vertex.unwrap();
        assert_eq!(get_type_show(&genv, ret_vtx), "String");
    }

    // Test 8: super call independence (SuperNode is not CallNode)
    #[test]
    fn test_super_call_independence() {
        let source = r#"
class Base
  def greet
    "hello"
  end
end

class Child < Base
  def greet
    super
  end
end
"#;
        let genv = analyze(source);

        // super is a SuperNode, not a CallNode, so ImplicitSelfCall should not be triggered.
        // Base#greet should still work.
        let info = genv
            .resolve_method(&Type::instance("Base"), "greet")
            .expect("Base#greet should be registered");
        assert!(info.return_vertex.is_some());

        let ret_vtx = info.return_vertex.unwrap();
        assert_eq!(get_type_show(&genv, ret_vtx), "String");
    }
}
