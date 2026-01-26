//! Definition Handlers - Processing Ruby class/method/module definitions
//!
//! This module is responsible for:
//! - Class definition scope management (class Foo ... end)
//! - Module definition scope management (module Bar ... end)
//! - Method definition scope management (def baz ... end)
//! - Extracting class/module names from AST nodes

use crate::env::GlobalEnv;

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
pub fn extract_class_name(class_node: &ruby_prism::ClassNode) -> String {
    if let Some(constant_read) = class_node.constant_path().as_constant_read_node() {
        String::from_utf8_lossy(constant_read.name().as_slice()).to_string()
    } else {
        "UnknownClass".to_string()
    }
}

/// Extract module name from ModuleNode
pub fn extract_module_name(module_node: &ruby_prism::ModuleNode) -> String {
    if let Some(constant_read) = module_node.constant_path().as_constant_read_node() {
        String::from_utf8_lossy(constant_read.name().as_slice()).to_string()
    } else {
        "UnknownModule".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
