//! Definition Handlers - Processing Ruby class/method/module definitions
//!
//! This module is responsible for:
//! - Class definition scope management (class Foo ... end)
//! - Module definition scope management (module Bar ... end)
//! - Method definition scope management (def baz ... end)
//! - Extracting class/module names from AST nodes (including qualified names like Api::User)

use crate::env::GlobalEnv;
use ruby_prism::Node;

/// Install class definition
pub fn install_class(genv: &mut GlobalEnv, class_name: String) {
    genv.enter_class(class_name);
}

/// Install module definition
pub fn install_module(genv: &mut GlobalEnv, module_name: String) {
    genv.enter_module(module_name);
}

/// Install method definition
pub fn install_method(genv: &mut GlobalEnv, method_name: String) {
    genv.enter_method(method_name);
}

/// Exit current scope (class, module, or method)
pub fn exit_scope(genv: &mut GlobalEnv) {
    genv.exit_scope();
}

/// Extract class name from ClassNode
/// Supports both simple names (User) and qualified names (Api::V1::User)
pub fn extract_class_name(class_node: &ruby_prism::ClassNode) -> String {
    extract_constant_path(&class_node.constant_path()).unwrap_or_else(|| "UnknownClass".to_string())
}

/// Extract module name from ModuleNode
/// Supports both simple names (Utils) and qualified names (Api::V1::Utils)
pub fn extract_module_name(module_node: &ruby_prism::ModuleNode) -> String {
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
fn extract_constant_path(node: &Node) -> Option<String> {
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
    use crate::parser::parse_ruby_source;

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
        let parse_result = parse_ruby_source(source, "test.rb".to_string()).unwrap();
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
        let parse_result = parse_ruby_source(source, "test.rb".to_string()).unwrap();
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
        let parse_result = parse_ruby_source(source, "test.rb".to_string()).unwrap();
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
        let parse_result = parse_ruby_source(source, "test.rb".to_string()).unwrap();
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
        let parse_result = parse_ruby_source(source, "test.rb".to_string()).unwrap();
        let root = parse_result.node();
        let program = root.as_program_node().unwrap();
        let stmt = program.statements().body().first().unwrap();
        let module_node = stmt.as_module_node().unwrap();

        let name = extract_module_name(&module_node);
        assert_eq!(name, "Api::V1");
    }
}
