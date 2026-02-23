//! Definition Handlers - Processing Ruby class/method/module definitions
//!
//! This module is responsible for:
//! - Class definition scope management (class Foo ... end)
//! - Module definition scope management (module Bar ... end)
//! - Method definition scope management (def baz ... end)
//! - Extracting class/module names from AST nodes (including qualified names like Api::User)

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};
use crate::types::Type;
use ruby_prism::Node;

use super::install::install_statements;
use super::parameters::install_parameters;

/// Process class definition node
pub(crate) fn process_class_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    class_node: &ruby_prism::ClassNode,
) -> Option<VertexId> {
    let class_name = extract_class_name(class_node);
    install_class(genv, class_name);

    if let Some(body) = class_node.body() {
        if let Some(statements) = body.as_statements_node() {
            install_statements(genv, lenv, changes, source, &statements);
        }
    }

    exit_scope(genv);
    None
}

/// Process module definition node
pub(crate) fn process_module_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    module_node: &ruby_prism::ModuleNode,
) -> Option<VertexId> {
    let module_name = extract_module_name(module_node);
    install_module(genv, module_name);

    if let Some(body) = module_node.body() {
        if let Some(statements) = body.as_statements_node() {
            install_statements(genv, lenv, changes, source, &statements);
        }
    }

    exit_scope(genv);
    None
}

/// Process method definition node
pub(crate) fn process_def_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    def_node: &ruby_prism::DefNode,
) -> Option<VertexId> {
    let method_name = String::from_utf8_lossy(def_node.name().as_slice()).to_string();

    // Check if this is a class method (def self.foo)
    let is_class_method = def_node
        .receiver()
        .map(|r| r.as_self_node().is_some())
        .unwrap_or(false);

    install_method(genv, method_name.clone());

    let merge_vtx = genv.scope_manager.current_method_return_vertex();

    // Process parameters BEFORE processing body
    let param_vtxs = if let Some(params_node) = def_node.parameters() {
        install_parameters(genv, lenv, changes, source, &params_node)
    } else {
        vec![]
    };

    let mut last_vtx = None;
    if let Some(body) = def_node.body() {
        if let Some(statements) = body.as_statements_node() {
            last_vtx = install_statements(genv, lenv, changes, source, &statements);
        }
    }

    // Connect last expression to merge vertex so that implicit return
    // (Ruby's last-expression-is-return-value) is included in the union type
    if let (Some(last), Some(merge)) = (last_vtx, merge_vtx) {
        genv.add_edge(last, merge);
    }

    // Register user-defined method with merge vertex as return vertex
    let return_vtx = merge_vtx.or(last_vtx);
    if let Some(ret_vtx) = return_vtx {
        let recv_type_name = genv.scope_manager.current_qualified_name();

        if let Some(name) = recv_type_name {
            let recv_type = if is_class_method {
                Type::singleton(&name)
            } else {
                Type::instance(&name)
            };
            genv.register_user_method(
                recv_type,
                &method_name,
                ret_vtx,
                param_vtxs,
            );
        }
    }

    exit_scope(genv);
    None
}

/// Install class definition
fn install_class(genv: &mut GlobalEnv, class_name: String) {
    genv.enter_class(class_name);
}

/// Install module definition
fn install_module(genv: &mut GlobalEnv, module_name: String) {
    genv.enter_module(module_name);
}

/// Install method definition
fn install_method(genv: &mut GlobalEnv, method_name: String) {
    genv.enter_method(method_name);
}

/// Exit current scope (class, module, or method)
fn exit_scope(genv: &mut GlobalEnv) {
    genv.exit_scope();
}

/// Extract class name from ClassNode
/// Supports both simple names (User) and qualified names (Api::V1::User)
fn extract_class_name(class_node: &ruby_prism::ClassNode) -> String {
    extract_constant_path(&class_node.constant_path()).unwrap_or_else(|| "UnknownClass".to_string())
}

/// Extract module name from ModuleNode
/// Supports both simple names (Utils) and qualified names (Api::V1::Utils)
fn extract_module_name(module_node: &ruby_prism::ModuleNode) -> String {
    extract_constant_path(&module_node.constant_path())
        .unwrap_or_else(|| "UnknownModule".to_string())
}

/// Extract constant path from a Node (handles both ConstantReadNode and ConstantPathNode)
///
/// Examples:
/// - `User` (ConstantReadNode) → "User"
/// - `Api::User` (ConstantPathNode) → "Api::User"
/// - `Api::V1::User` (nested ConstantPathNode) → "Api::V1::User"
/// - `::Api::User` (absolute path with COLON3) → "Api::User"
pub(crate) fn extract_constant_path(node: &Node) -> Option<String> {
    // Simple constant read: `User`
    if let Some(constant_read) = node.as_constant_read_node() {
        return Some(String::from_utf8_lossy(constant_read.name().as_slice()).to_string());
    }

    // Constant path: `Api::User` or `Api::V1::User`
    if let Some(constant_path) = node.as_constant_path_node() {
        // name() returns Option<ConstantId>, use as_slice() to get &[u8]
        let name = constant_path
            .name()
            .map(|id| String::from_utf8_lossy(id.as_slice()).to_string())?;

        // Get parent path if exists
        if let Some(parent_node) = constant_path.parent() {
            if let Some(parent_path) = extract_constant_path(&parent_node) {
                return Some(format!("{}::{}", parent_path, name));
            }
        }

        // No parent (absolute path like `::User`)
        return Some(name);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::ChangeSet;
    use crate::parser::ParseSession;
    use crate::types::Type;

    #[test]
    fn test_enter_exit_class_scope() {
        let mut genv = GlobalEnv::new();

        install_class(&mut genv, "User".to_string());
        assert_eq!(
            genv.scope_manager.current_class_name(),
            Some("User".to_string())
        );

        exit_scope(&mut genv);
        assert_eq!(genv.scope_manager.current_class_name(), None);
    }

    #[test]
    fn test_enter_exit_module_scope() {
        let mut genv = GlobalEnv::new();

        install_module(&mut genv, "Utils".to_string());
        assert_eq!(
            genv.scope_manager.current_module_name(),
            Some("Utils".to_string())
        );

        exit_scope(&mut genv);
        assert_eq!(genv.scope_manager.current_module_name(), None);
    }

    #[test]
    fn test_nested_method_scope() {
        let mut genv = GlobalEnv::new();

        install_class(&mut genv, "User".to_string());
        install_method(&mut genv, "greet".to_string());

        // Still in User class context
        assert_eq!(
            genv.scope_manager.current_class_name(),
            Some("User".to_string())
        );

        exit_scope(&mut genv); // exit method
        exit_scope(&mut genv); // exit class

        assert_eq!(genv.scope_manager.current_class_name(), None);
    }

    #[test]
    fn test_method_in_module() {
        let mut genv = GlobalEnv::new();

        install_module(&mut genv, "Helpers".to_string());
        install_method(&mut genv, "format".to_string());

        // Should find module context from within method
        assert_eq!(
            genv.scope_manager.current_module_name(),
            Some("Helpers".to_string())
        );

        exit_scope(&mut genv); // exit method
        exit_scope(&mut genv); // exit module

        assert_eq!(genv.scope_manager.current_module_name(), None);
    }

    #[test]
    fn test_extract_simple_class_name() {
        let source = "class User; end";
        let session = ParseSession::new();
        let parse_result = session.parse_source(source, "test.rb").unwrap();
        let root = parse_result.node();
        let program = root.as_program_node().unwrap();
        let stmt = program.statements().body().first().unwrap();
        let class_node = stmt.as_class_node().unwrap();

        let name = extract_class_name(&class_node);
        assert_eq!(name, "User");
    }

    #[test]
    fn test_extract_qualified_class_name() {
        let source = "class Api::User; end";
        let session = ParseSession::new();
        let parse_result = session.parse_source(source, "test.rb").unwrap();
        let root = parse_result.node();
        let program = root.as_program_node().unwrap();
        let stmt = program.statements().body().first().unwrap();
        let class_node = stmt.as_class_node().unwrap();

        let name = extract_class_name(&class_node);
        assert_eq!(name, "Api::User");
    }

    #[test]
    fn test_extract_deeply_qualified_class_name() {
        let source = "class Api::V1::Admin::User; end";
        let session = ParseSession::new();
        let parse_result = session.parse_source(source, "test.rb").unwrap();
        let root = parse_result.node();
        let program = root.as_program_node().unwrap();
        let stmt = program.statements().body().first().unwrap();
        let class_node = stmt.as_class_node().unwrap();

        let name = extract_class_name(&class_node);
        assert_eq!(name, "Api::V1::Admin::User");
    }

    #[test]
    fn test_extract_simple_module_name() {
        let source = "module Utils; end";
        let session = ParseSession::new();
        let parse_result = session.parse_source(source, "test.rb").unwrap();
        let root = parse_result.node();
        let program = root.as_program_node().unwrap();
        let stmt = program.statements().body().first().unwrap();
        let module_node = stmt.as_module_node().unwrap();

        let name = extract_module_name(&module_node);
        assert_eq!(name, "Utils");
    }

    #[test]
    fn test_extract_qualified_module_name() {
        let source = "module Api::V1; end";
        let session = ParseSession::new();
        let parse_result = session.parse_source(source, "test.rb").unwrap();
        let root = parse_result.node();
        let program = root.as_program_node().unwrap();
        let stmt = program.statements().body().first().unwrap();
        let module_node = stmt.as_module_node().unwrap();

        let name = extract_module_name(&module_node);
        assert_eq!(name, "Api::V1");
    }

    #[test]
    fn test_process_def_node_registers_user_method() {
        let source = "class User; def name; \"Alice\"; end; end";
        let session = ParseSession::new();
        let parse_result = session.parse_source(source, "test.rb").unwrap();
        let root = parse_result.node();
        let program = root.as_program_node().unwrap();

        let mut genv = GlobalEnv::new();
        let mut lenv = LocalEnv::new();
        let mut changes = ChangeSet::new();

        let stmt = program.statements().body().first().unwrap();
        let class_node = stmt.as_class_node().unwrap();
        process_class_node(&mut genv, &mut lenv, &mut changes, source, &class_node);

        // User#name should be registered as a user-defined method
        let info = genv
            .resolve_method(&Type::instance("User"), "name")
            .expect("User#name should be registered");
        assert!(info.return_vertex.is_some());
    }

    #[test]
    fn test_qualified_name_method_registration() {
        let source = r#"
module Api
  module V1
    class User
      def name
        "Alice"
      end
    end
  end
end
"#;
        let session = ParseSession::new();
        let parse_result = session.parse_source(source, "test.rb").unwrap();
        let root = parse_result.node();
        let program = root.as_program_node().unwrap();

        let mut genv = GlobalEnv::new();
        let mut lenv = LocalEnv::new();
        let mut changes = ChangeSet::new();

        for stmt in &program.statements().body() {
            crate::analyzer::install::install_node(&mut genv, &mut lenv, &mut changes, source, &stmt);
        }

        // Method should be registered with qualified name "Api::V1::User"
        let info = genv
            .resolve_method(&Type::instance("Api::V1::User"), "name")
            .expect("Api::V1::User#name should be registered");
        assert!(info.return_vertex.is_some());

        // Should NOT be registered with simple name "User"
        assert!(
            genv.resolve_method(&Type::instance("User"), "name").is_none(),
            "User#name should not exist — method should be registered under qualified name"
        );
    }

    #[test]
    fn test_same_class_name_different_namespace() {
        let source = r#"
module Api
  class User
    def name
      "api_user"
    end
  end
end

module Admin
  class User
    def name
      "admin_user"
    end
  end
end
"#;
        let session = ParseSession::new();
        let parse_result = session.parse_source(source, "test.rb").unwrap();
        let root = parse_result.node();
        let program = root.as_program_node().unwrap();

        let mut genv = GlobalEnv::new();
        let mut lenv = LocalEnv::new();
        let mut changes = ChangeSet::new();

        for stmt in &program.statements().body() {
            crate::analyzer::install::install_node(&mut genv, &mut lenv, &mut changes, source, &stmt);
        }

        // Both should be registered separately
        let api_info = genv
            .resolve_method(&Type::instance("Api::User"), "name")
            .expect("Api::User#name should be registered");
        assert!(api_info.return_vertex.is_some());

        let admin_info = genv
            .resolve_method(&Type::instance("Admin::User"), "name")
            .expect("Admin::User#name should be registered");
        assert!(admin_info.return_vertex.is_some());

        // Simple "User" should not resolve
        assert!(
            genv.resolve_method(&Type::instance("User"), "name").is_none(),
            "User#name should not exist — both are under qualified names"
        );
    }

    #[test]
    fn test_class_method_registration() {
        let source = r#"
class User
  def self.create
    "created"
  end
end
"#;
        let session = ParseSession::new();
        let parse_result = session.parse_source(source, "test.rb").unwrap();
        let root = parse_result.node();
        let program = root.as_program_node().unwrap();

        let mut genv = GlobalEnv::new();
        let mut lenv = LocalEnv::new();
        let mut changes = ChangeSet::new();

        for stmt in &program.statements().body() {
            crate::analyzer::install::install_node(&mut genv, &mut lenv, &mut changes, source, &stmt);
        }

        // def self.create should be registered as singleton method
        let info = genv
            .resolve_method(&Type::singleton("User"), "create")
            .expect("User.create should be registered as singleton method");
        assert!(info.return_vertex.is_some());
    }

    #[test]
    fn test_class_method_with_params() {
        let source = r#"
class User
  def self.find(id)
    "user"
  end
end
"#;
        let session = ParseSession::new();
        let parse_result = session.parse_source(source, "test.rb").unwrap();
        let root = parse_result.node();
        let program = root.as_program_node().unwrap();

        let mut genv = GlobalEnv::new();
        let mut lenv = LocalEnv::new();
        let mut changes = ChangeSet::new();

        for stmt in &program.statements().body() {
            crate::analyzer::install::install_node(&mut genv, &mut lenv, &mut changes, source, &stmt);
        }

        let info = genv
            .resolve_method(&Type::singleton("User"), "find")
            .expect("User.find should be registered");
        assert!(info.return_vertex.is_some());
        assert_eq!(info.param_vertices.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_class_method_in_qualified_namespace() {
        let source = r#"
module Api
  class User
    def self.create
      "created"
    end
  end
end
"#;
        let session = ParseSession::new();
        let parse_result = session.parse_source(source, "test.rb").unwrap();
        let root = parse_result.node();
        let program = root.as_program_node().unwrap();

        let mut genv = GlobalEnv::new();
        let mut lenv = LocalEnv::new();
        let mut changes = ChangeSet::new();

        for stmt in &program.statements().body() {
            crate::analyzer::install::install_node(&mut genv, &mut lenv, &mut changes, source, &stmt);
        }

        let info = genv
            .resolve_method(&Type::singleton("Api::User"), "create")
            .expect("Api::User.create should be registered");
        assert!(info.return_vertex.is_some());
    }

    #[test]
    fn test_class_method_not_registered_as_instance() {
        let source = r#"
class User
  def self.create
    "created"
  end
end
"#;
        let session = ParseSession::new();
        let parse_result = session.parse_source(source, "test.rb").unwrap();
        let root = parse_result.node();
        let program = root.as_program_node().unwrap();

        let mut genv = GlobalEnv::new();
        let mut lenv = LocalEnv::new();
        let mut changes = ChangeSet::new();

        for stmt in &program.statements().body() {
            crate::analyzer::install::install_node(&mut genv, &mut lenv, &mut changes, source, &stmt);
        }

        // def self.create should NOT be registered as instance method
        assert!(
            genv.resolve_method(&Type::instance("User"), "create").is_none(),
            "User#create should not exist — it's a class method"
        );
    }

    #[test]
    fn test_non_self_receiver_not_treated_as_class_method() {
        let source = r#"
class User
  def other.foo
    "test"
  end
end
"#;
        let session = ParseSession::new();
        let parse_result = session.parse_source(source, "test.rb").unwrap();
        let root = parse_result.node();
        let program = root.as_program_node().unwrap();

        let mut genv = GlobalEnv::new();
        let mut lenv = LocalEnv::new();
        let mut changes = ChangeSet::new();

        for stmt in &program.statements().body() {
            crate::analyzer::install::install_node(&mut genv, &mut lenv, &mut changes, source, &stmt);
        }

        // def other.foo should NOT be registered as singleton method
        assert!(
            genv.resolve_method(&Type::singleton("User"), "foo").is_none(),
            "User.foo should not exist — receiver is not self"
        );
    }

    #[test]
    fn test_class_method_return_type_inference() {
        let source = r#"
class User
  def self.create
    "created"
  end
end
"#;
        let session = ParseSession::new();
        let parse_result = session.parse_source(source, "test.rb").unwrap();
        let root = parse_result.node();
        let program = root.as_program_node().unwrap();

        let mut genv = GlobalEnv::new();
        let mut lenv = LocalEnv::new();
        let mut changes = ChangeSet::new();

        for stmt in &program.statements().body() {
            crate::analyzer::install::install_node(&mut genv, &mut lenv, &mut changes, source, &stmt);
        }

        let info = genv
            .resolve_method(&Type::singleton("User"), "create")
            .expect("User.create should be registered");
        let ret_vtx = info.return_vertex.expect("should have return vertex");

        // Run solver to propagate types
        genv.apply_changes(changes);
        genv.run_all();

        let vertex = genv.get_vertex(ret_vtx).or_else(|| {
            // return vertex might be a source
            None
        });
        if let Some(v) = vertex {
            assert_eq!(v.show(), "String");
        } else {
            // Check if it's a source
            let src = genv.get_source(ret_vtx).expect("should have source or vertex");
            assert_eq!(src.ty, Type::string());
        }
    }

    #[test]
    fn test_class_method_in_reopened_class() {
        let source = r#"
class User
  def self.create
    "created"
  end
end

class User
  def self.destroy
    "destroyed"
  end
end
"#;
        let session = ParseSession::new();
        let parse_result = session.parse_source(source, "test.rb").unwrap();
        let root = parse_result.node();
        let program = root.as_program_node().unwrap();

        let mut genv = GlobalEnv::new();
        let mut lenv = LocalEnv::new();
        let mut changes = ChangeSet::new();

        for stmt in &program.statements().body() {
            crate::analyzer::install::install_node(&mut genv, &mut lenv, &mut changes, source, &stmt);
        }

        // Both class methods should be registered
        assert!(
            genv.resolve_method(&Type::singleton("User"), "create").is_some(),
            "User.create should be registered"
        );
        assert!(
            genv.resolve_method(&Type::singleton("User"), "destroy").is_some(),
            "User.destroy should be registered"
        );
    }

    #[test]
    fn test_class_method_param_type_propagation() {
        let source = r#"
class User
  def self.find(id)
    id
  end
end

User.find(42)
"#;
        let session = ParseSession::new();
        let parse_result = session.parse_source(source, "test.rb").unwrap();
        let root = parse_result.node();
        let program = root.as_program_node().unwrap();

        let mut genv = GlobalEnv::new();
        let mut lenv = LocalEnv::new();
        let mut changes = ChangeSet::new();

        for stmt in &program.statements().body() {
            crate::analyzer::install::install_node(&mut genv, &mut lenv, &mut changes, source, &stmt);
        }

        let info = genv
            .resolve_method(&Type::singleton("User"), "find")
            .expect("User.find should be registered");
        let param_vtxs = info.param_vertices.as_ref().expect("should have param vertices");
        assert_eq!(param_vtxs.len(), 1);

        let param_vtx = param_vtxs[0];

        // Run solver to propagate argument types
        genv.apply_changes(changes);
        genv.run_all();

        // Parameter should have Integer type propagated from call site
        let vertex = genv.get_vertex(param_vtx).expect("param vertex should exist");
        assert_eq!(vertex.show(), "Integer");
    }
}
