//! Method registration and resolution

use crate::types::Type;
use std::collections::HashMap;

/// Method information
#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub return_type: Type,
}

/// Registry for method definitions
#[derive(Debug, Default)]
pub struct MethodRegistry {
    methods: HashMap<(Type, String), MethodInfo>,
}

impl MethodRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            methods: HashMap::new(),
        }
    }

    /// Register a method for a receiver type
    pub fn register(&mut self, recv_ty: Type, method_name: &str, ret_ty: Type) {
        self.methods.insert(
            (recv_ty, method_name.to_string()),
            MethodInfo {
                return_type: ret_ty,
            },
        );
    }

    /// Resolve a method for a receiver type
    pub fn resolve(&self, recv_ty: &Type, method_name: &str) -> Option<&MethodInfo> {
        self.methods
            .get(&(recv_ty.clone(), method_name.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_resolve() {
        let mut registry = MethodRegistry::new();
        registry.register(Type::string(), "length", Type::integer());

        let info = registry.resolve(&Type::string(), "length").unwrap();
        assert!(
            matches!(info.return_type, Type::Instance { ref class_name, .. } if class_name == "Integer")
        );
    }

    #[test]
    fn test_resolve_not_found() {
        let registry = MethodRegistry::new();
        assert!(registry.resolve(&Type::string(), "unknown").is_none());
    }
}
