//! Method registration and resolution

use crate::types::Type;
use std::collections::HashMap;

/// Method information
#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub return_type: Type,
    pub block_param_types: Option<Vec<Type>>,
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
        self.register_with_block(recv_ty, method_name, ret_ty, None);
    }

    /// Register a method with block parameter types
    pub fn register_with_block(
        &mut self,
        recv_ty: Type,
        method_name: &str,
        ret_ty: Type,
        block_param_types: Option<Vec<Type>>,
    ) {
        self.methods.insert(
            (recv_ty, method_name.to_string()),
            MethodInfo {
                return_type: ret_ty,
                block_param_types,
            },
        );
    }

    /// Resolve a method for a receiver type
    ///
    /// For generic types like `Array[Integer]`, first tries exact match,
    /// then falls back to base class match (`Array`).
    pub fn resolve(&self, recv_ty: &Type, method_name: &str) -> Option<&MethodInfo> {
        // First, try exact match
        if let Some(info) = self
            .methods
            .get(&(recv_ty.clone(), method_name.to_string()))
        {
            return Some(info);
        }

        // For generic types, fall back to base class
        if let Type::Generic { name, .. } = recv_ty {
            let base_type = Type::Instance { name: name.clone() };
            return self.methods.get(&(base_type, method_name.to_string()));
        }

        None
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
        assert_eq!(info.return_type.base_class_name(), Some("Integer"));
    }

    #[test]
    fn test_resolve_not_found() {
        let registry = MethodRegistry::new();
        assert!(registry.resolve(&Type::string(), "unknown").is_none());
    }
}
