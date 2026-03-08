//! Node Dispatch - Dispatch AST nodes to appropriate handlers
//!
//! This module handles the pattern matching of Ruby AST nodes
//! and dispatches them to specialized handlers.

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{BlockParameterTypeBox, ChangeSet, VertexId};
use crate::source_map::SourceLocation;
use crate::types::Type;
use ruby_prism::Node;

use super::bytes_to_name;
use super::calls::install_method_call;
use super::variables::{
    install_ivar_read, install_ivar_write, install_local_var_read, install_local_var_write,
    install_self,
};

/// Kind of attr_* declaration
#[derive(Debug, Clone, Copy)]
pub enum AttrKind {
    Reader,
    Writer,
    Accessor,
}

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
    /// attr_reader / attr_writer / attr_accessor declaration
    AttrDeclaration {
        kind: AttrKind,
        attr_names: Vec<String>,
    },
}

/// First pass: check if node can be handled immediately without child processing
///
/// Note: Literals (including Array) are handled in install.rs via install_literal
/// because Array literals need child processing for element type inference.
pub fn dispatch_simple(genv: &mut GlobalEnv, lenv: &mut LocalEnv, node: &Node) -> DispatchResult {
    // Instance variable read: @name
    if let Some(ivar_read) = node.as_instance_variable_read_node() {
        let ivar_name = bytes_to_name(ivar_read.name().as_slice());
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
        let var_name = bytes_to_name(read_node.name().as_slice());
        return match install_local_var_read(lenv, &var_name) {
            Some(vtx) => DispatchResult::Vertex(vtx),
            None => DispatchResult::NotHandled,
        };
    }

    // ConstantReadNode: User → Type::Singleton("User") or Type::Singleton("Api::User")
    if let Some(const_read) = node.as_constant_read_node() {
        let name = bytes_to_name(const_read.name().as_slice());
        let resolved_name = genv.scope_manager.lookup_constant(&name)
            .unwrap_or(name);
        let vtx = genv.new_source(Type::singleton(&resolved_name));
        return DispatchResult::Vertex(vtx);
    }

    // ConstantPathNode: Api::User → Type::Singleton("Api::User")
    if node.as_constant_path_node().is_some() {
        if let Some(name) = super::definitions::extract_constant_path(node) {
            let vtx = genv.new_source(Type::singleton(&name));
            return DispatchResult::Vertex(vtx);
        }
    }

    DispatchResult::NotHandled
}

/// Extract symbol names from attr_* arguments (e.g., `attr_reader :name, :email`)
fn extract_symbol_names(call_node: &ruby_prism::CallNode) -> Vec<String> {
    call_node
        .arguments()
        .map(|args| {
            args.arguments()
                .iter()
                .filter_map(|arg| {
                    arg.as_symbol_node().map(|sym| {
                        bytes_to_name(sym.unescaped())
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Check if node needs child processing
pub fn dispatch_needs_child<'a>(node: &Node<'a>, source: &str) -> Option<NeedsChildKind<'a>> {
    // Instance variable write: @name = value
    if let Some(ivar_write) = node.as_instance_variable_write_node() {
        let ivar_name = bytes_to_name(ivar_write.name().as_slice());
        return Some(NeedsChildKind::IvarWrite {
            ivar_name,
            value: ivar_write.value(),
        });
    }

    // Local variable write: x = value
    if let Some(write_node) = node.as_local_variable_write_node() {
        let var_name = bytes_to_name(write_node.name().as_slice());
        return Some(NeedsChildKind::LocalVarWrite {
            var_name,
            value: write_node.value(),
        });
    }

    // Method call: x.upcase, x.each { |i| ... }, or name (implicit self)
    if let Some(call_node) = node.as_call_node() {
        let method_name = bytes_to_name(call_node.name().as_slice());
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

            if let Some(kind) = match method_name.as_str() {
                "attr_reader" => Some(AttrKind::Reader),
                "attr_writer" => Some(AttrKind::Writer),
                "attr_accessor" => Some(AttrKind::Accessor),
                _ => None,
            } {
                let attr_names = extract_symbol_names(&call_node);
                if !attr_names.is_empty() {
                    return Some(NeedsChildKind::AttrDeclaration { kind, attr_names });
                }
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
            // Use qualified name to match method registration in definitions.rs
            let recv_vtx = if let Some(name) = genv.scope_manager.current_qualified_name() {
                genv.new_source(Type::instance(&name))
            } else {
                genv.new_source(Type::instance("Object"))
            };
            process_method_call_common(
                genv, lenv, changes, source, recv_vtx, method_name, location, block, arguments,
            )
        }
        NeedsChildKind::AttrDeclaration { kind, attr_names } => {
            super::attributes::process_attr_declaration(genv, kind, attr_names);
            None
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
    if method_name == "!" {
        return Some(super::operators::process_not_operator(genv));
    }

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

        // Method registered with qualified name "Api::V1::User"
        let info = genv
            .resolve_method(&Type::instance("Api::V1::User"), "greet")
            .expect("Api::V1::User#greet should be registered");
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
            .resolve_method(&Type::singleton("Utils"), "run")
            .expect("Utils.run should be registered");
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

    // Test 7: attr_reader basic — getter type resolution
    #[test]
    fn test_attr_reader_basic() {
        let source = r#"
class User
  attr_reader :name

  def initialize
    @name = "Alice"
  end
end
"#;
        let genv = analyze(source);

        // User#name should be registered and resolve to String via @name
        let info = genv
            .resolve_method(&Type::instance("User"), "name")
            .expect("User#name should be registered");
        assert!(info.return_vertex.is_some());
        let ret_vtx = info.return_vertex.unwrap();
        assert_eq!(get_type_show(&genv, ret_vtx), "String");

        // Other methods still work
        let greet_src = r#"
class User
  attr_reader :name

  def greet
    "hello"
  end
end
"#;
        let genv2 = analyze(greet_src);
        let info2 = genv2
            .resolve_method(&Type::instance("User"), "greet")
            .expect("User#greet should be registered");
        assert_eq!(get_type_show(&genv2, info2.return_vertex.unwrap()), "String");
    }

    // Test 8: attr_reader + self.name method call
    #[test]
    fn test_attr_reader_with_self_call() {
        let source = r#"
class User
  attr_reader :name

  def initialize
    @name = "Alice"
  end

  def greet
    self.name
  end
end
"#;
        let genv = analyze(source);

        // User#greet should resolve to String via User#name → @name
        let info = genv
            .resolve_method(&Type::instance("User"), "greet")
            .expect("User#greet should be registered");
        assert!(info.return_vertex.is_some());
        let ret_vtx = info.return_vertex.unwrap();
        assert_eq!(get_type_show(&genv, ret_vtx), "String");
    }

    // Test 9: attr_reader + receiverless name call (proposal D integration)
    #[test]
    fn test_attr_reader_receiverless_call() {
        let source = r#"
class User
  attr_reader :name

  def initialize
    @name = "Alice"
  end

  def greet
    name
  end
end
"#;
        let genv = analyze(source);

        // User#greet should resolve to String via implicit self → User#name → @name
        let info = genv
            .resolve_method(&Type::instance("User"), "greet")
            .expect("User#greet should be registered");
        assert!(info.return_vertex.is_some());
        let ret_vtx = info.return_vertex.unwrap();
        assert_eq!(get_type_show(&genv, ret_vtx), "String");
    }

    // Test 10: attr_accessor — both getter and setter
    #[test]
    fn test_attr_accessor() {
        let source = r#"
class User
  attr_accessor :age

  def initialize
    @age = 30
  end
end
"#;
        let genv = analyze(source);

        // User#age (getter) should be registered and resolve to Integer
        let getter = genv
            .resolve_method(&Type::instance("User"), "age")
            .expect("User#age getter should be registered");
        assert!(getter.return_vertex.is_some());
        assert_eq!(get_type_show(&genv, getter.return_vertex.unwrap()), "Integer");

        // User#age= (setter) should also be registered
        let setter = genv
            .resolve_method(&Type::instance("User"), "age=")
            .expect("User#age= setter should be registered");
        assert!(setter.return_vertex.is_some());
    }

    // Test 11: multiple attributes in single declaration
    #[test]
    fn test_attr_reader_multiple() {
        let source = r#"
class User
  attr_reader :name, :email

  def initialize
    @name = "Alice"
    @email = "alice@test.com"
  end
end
"#;
        let genv = analyze(source);

        let name_info = genv
            .resolve_method(&Type::instance("User"), "name")
            .expect("User#name should be registered");
        assert_eq!(get_type_show(&genv, name_info.return_vertex.unwrap()), "String");

        let email_info = genv
            .resolve_method(&Type::instance("User"), "email")
            .expect("User#email should be registered");
        assert_eq!(get_type_show(&genv, email_info.return_vertex.unwrap()), "String");
    }

    // Test 12: attr_reader in nested class
    #[test]
    fn test_attr_reader_nested_class() {
        let source = r#"
module Api
  class User
    attr_reader :name

    def initialize
      @name = "Alice"
    end
  end
end
"#;
        let genv = analyze(source);

        // Registered with qualified name "Api::User"
        let info = genv
            .resolve_method(&Type::instance("Api::User"), "name")
            .expect("Api::User#name should be registered");
        assert!(info.return_vertex.is_some());
        assert_eq!(get_type_show(&genv, info.return_vertex.unwrap()), "String");
    }

    // Test 13: attr_writer only — setter registered, getter not
    #[test]
    fn test_attr_writer_only() {
        let source = r#"
class User
  attr_writer :name
end
"#;
        let genv = analyze(source);

        // User#name= should be registered
        let setter = genv.resolve_method(&Type::instance("User"), "name=");
        assert!(setter.is_some(), "User#name= should be registered");

        // User#name (getter) should NOT be registered
        let getter = genv.resolve_method(&Type::instance("User"), "name");
        assert!(getter.is_none(), "User#name getter should NOT be registered for attr_writer");
    }

    // Test 14: attr_reader with no assignment (empty vertex)
    #[test]
    fn test_attr_reader_unassigned() {
        let source = r#"
class User
  attr_reader :unknown
end
"#;
        let genv = analyze(source);

        // User#unknown should be registered (with empty vertex)
        let info = genv
            .resolve_method(&Type::instance("User"), "unknown")
            .expect("User#unknown should be registered");
        assert!(info.return_vertex.is_some());
    }

    // Test 15: super call independence (SuperNode is not CallNode)
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

    // Test 16: User.new → instance(User)
    #[test]
    fn test_constant_read_user_new() {
        let source = r#"
class User
  def name
    "Alice"
  end
end

x = User.new
"#;
        let genv = analyze(source);
        assert!(
            genv.type_errors.is_empty(),
            "User.new should not produce type errors: {:?}",
            genv.type_errors
        );
    }

    // Test 17: User.new.name → String
    #[test]
    fn test_constant_read_user_new_method_chain() {
        let source = r#"
class User
  def name
    "Alice"
  end
end

x = User.new.name
"#;
        let genv = analyze(source);
        assert!(
            genv.type_errors.is_empty(),
            "User.new.name should not produce type errors: {:?}",
            genv.type_errors
        );
    }

    // Test 18: Api::User.new → instance(Api::User) (ConstantPathNode)
    #[test]
    fn test_constant_path_qualified_new() {
        let source = r#"
class Api::User
  def name
    "Alice"
  end
end

x = Api::User.new
"#;
        let genv = analyze(source);
        assert!(
            genv.type_errors.is_empty(),
            "Api::User.new should not produce type errors: {:?}",
            genv.type_errors
        );
    }

    // Test 19: User.new("Alice") → initialize parameter propagation
    #[test]
    fn test_constant_read_new_with_initialize_params() {
        let source = r#"
class User
  def initialize(name)
    @name = name
  end
end

x = User.new("Alice")
"#;
        let genv = analyze(source);
        assert!(genv.type_errors.is_empty());
    }

    // Test 20: user = User.new; user.name → String
    #[test]
    fn test_constant_read_assign_and_call() {
        let source = r#"
class User
  def name
    "Alice"
  end
end

user = User.new
user.name
"#;
        let genv = analyze(source);
        assert!(
            genv.type_errors.is_empty(),
            "user = User.new; user.name should not produce type errors: {:?}",
            genv.type_errors
        );
    }

    // Test 21: User.some_method should not produce type error (Singleton error suppression)
    #[test]
    fn test_constant_read_no_false_positive() {
        let source = r#"
class User
  def name
    "Alice"
  end
end

User.some_method
"#;
        let genv = analyze(source);
        assert!(
            genv.type_errors.is_empty(),
            "User.some_method should not produce type errors (Singleton suppression): {:?}",
            genv.type_errors
        );
    }

    // Test: ConstantPathNode.new resolves with qualified method name
    #[test]
    fn test_constant_path_new_with_qualified_method() {
        let source = r#"
module Api
  class User
    def name
      "Alice"
    end
  end
end

Api::User.new.name
"#;
        let genv = analyze(source);
        // Api::User.new.name should resolve correctly — no type errors
        assert!(
            genv.type_errors.is_empty(),
            "Api::User.new.name should not produce type errors: {:?}",
            genv.type_errors
        );
    }

    // Test 23: ConstantReadNode inside module resolves to qualified name
    #[test]
    fn test_constant_read_inside_module_resolves_qualified() {
        let source = r#"
module Api
  class User
    def name
      "Alice"
    end
  end

  class Service
    def run
      User.new.name
    end
  end
end
"#;
        let genv = analyze(source);
        assert!(
            genv.type_errors.is_empty(),
            "User.new inside module Api should resolve to Api::User: {:?}",
            genv.type_errors
        );
    }

    // Test 24: ConstantReadNode in deeply nested modules
    #[test]
    fn test_constant_read_deeply_nested() {
        let source = r#"
module Api
  module V1
    class User
      def name
        "Alice"
      end
    end

    class Service
      def run
        User.new.name
      end
    end
  end
end
"#;
        let genv = analyze(source);
        assert!(
            genv.type_errors.is_empty(),
            "User.new inside Api::V1 should resolve to Api::V1::User: {:?}",
            genv.type_errors
        );
    }

    // Test 25: Same constant name in different modules
    #[test]
    fn test_constant_read_same_name_different_modules() {
        let source = r#"
module Api
  class User
    def name; "Api User"; end
  end
end

module Admin
  class User
    def name; "Admin User"; end
  end

  class Service
    def run
      User.new.name
    end
  end
end
"#;
        let genv = analyze(source);
        assert!(
            genv.type_errors.is_empty(),
            "User.new inside Admin should resolve to Admin::User: {:?}",
            genv.type_errors
        );
    }
}
