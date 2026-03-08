//! Method registration and resolution

use std::collections::HashMap;

use crate::graph::VertexId;
use crate::types::Type;
use smallvec::SmallVec;

const OBJECT_CLASS: &str = "Object";
const KERNEL_MODULE: &str = "Kernel";

/// Method information
#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub return_type: Type,
    pub block_param_types: Option<Vec<Type>>,
    pub return_vertex: Option<VertexId>,
    pub param_vertices: Option<Vec<VertexId>>,
    pub keyword_param_vertices: Option<HashMap<String, VertexId>>,
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
                return_vertex: None,
                param_vertices: None,
                keyword_param_vertices: None,
            },
        );
    }

    /// Register a user-defined method (return type resolved via graph)
    pub fn register_user_method(
        &mut self,
        recv_ty: Type,
        method_name: &str,
        return_vertex: VertexId,
        param_vertices: Vec<VertexId>,
        keyword_param_vertices: Option<HashMap<String, VertexId>>,
    ) {
        self.methods.insert(
            (recv_ty, method_name.to_string()),
            MethodInfo {
                return_type: Type::Bot,
                block_param_types: None,
                return_vertex: Some(return_vertex),
                param_vertices: Some(param_vertices),
                keyword_param_vertices,
            },
        );
    }

    /// Build the method resolution order (MRO) fallback chain for a receiver type.
    ///
    /// Returns a list of types to search in order:
    /// 1. Exact receiver type
    /// 2. Generic → base class (e.g., Array[Integer] → Array)
    /// 3. Object (for Instance/Generic types only)
    /// 4. Kernel (for Instance/Generic types only)
    fn fallback_chain(recv_ty: &Type) -> SmallVec<[Type; 4]> {
        let mut chain = SmallVec::new();
        chain.push(recv_ty.clone());

        if let Type::Generic { name, .. } = recv_ty {
            chain.push(Type::Instance { name: name.clone() });
        }

        // NOTE: Kernel is a module, not a class. Represented as Type::Instance
        // due to lack of Type::Module variant.
        if matches!(recv_ty, Type::Instance { .. } | Type::Generic { .. }) {
            chain.push(Type::instance(OBJECT_CLASS));
            chain.push(Type::instance(KERNEL_MODULE));
        }

        chain
    }

    /// Resolve a method for a receiver type
    ///
    /// Searches the MRO fallback chain: exact type → base class (for generics) → Object → Kernel.
    /// For non-instance types (Singleton, Nil, Union, Bot), only exact match is attempted.
    pub fn resolve(&self, recv_ty: &Type, method_name: &str) -> Option<&MethodInfo> {
        let method_key = method_name.to_string();
        Self::fallback_chain(recv_ty)
            .into_iter()
            .find_map(|ty| self.methods.get(&(ty, method_key.clone())))
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

    #[test]
    fn test_register_user_method_and_resolve() {
        let mut registry = MethodRegistry::new();
        let return_vtx = VertexId(42);
        registry.register_user_method(Type::instance("User"), "name", return_vtx, vec![], None);

        let info = registry.resolve(&Type::instance("User"), "name").unwrap();
        assert_eq!(info.return_vertex, Some(VertexId(42)));
        assert_eq!(info.return_type, Type::Bot);
    }

    #[test]
    fn test_register_user_method_with_param_vertices() {
        let mut registry = MethodRegistry::new();
        let return_vtx = VertexId(10);
        let param_vtxs = vec![VertexId(20), VertexId(21)];
        registry.register_user_method(
            Type::instance("Calc"),
            "add",
            return_vtx,
            param_vtxs,
            None,
        );

        let info = registry.resolve(&Type::instance("Calc"), "add").unwrap();
        assert_eq!(info.return_vertex, Some(VertexId(10)));
        let pvs = info.param_vertices.as_ref().unwrap();
        assert_eq!(pvs.len(), 2);
        assert_eq!(pvs[0], VertexId(20));
        assert_eq!(pvs[1], VertexId(21));
    }

    // --- Object/Kernel fallback ---

    #[test]
    fn test_resolve_falls_back_to_object() {
        let mut registry = MethodRegistry::new();
        registry.register(Type::instance("Object"), "nil?", Type::instance("TrueClass"));
        let info = registry.resolve(&Type::instance("CustomClass"), "nil?").unwrap();
        assert_eq!(info.return_type.base_class_name(), Some("TrueClass"));
    }

    #[test]
    fn test_resolve_falls_back_to_kernel() {
        let mut registry = MethodRegistry::new();
        registry.register(Type::instance("Kernel"), "puts", Type::Nil);
        let info = registry.resolve(&Type::instance("MyApp"), "puts").unwrap();
        assert_eq!(info.return_type, Type::Nil);
    }

    #[test]
    fn test_resolve_object_before_kernel() {
        let mut registry = MethodRegistry::new();
        registry.register(Type::instance("Object"), "to_s", Type::string());
        registry.register(Type::instance("Kernel"), "to_s", Type::integer());
        let info = registry.resolve(&Type::instance("Anything"), "to_s").unwrap();
        assert_eq!(info.return_type.base_class_name(), Some("String"));
    }

    #[test]
    fn test_resolve_exact_match_over_fallback() {
        let mut registry = MethodRegistry::new();
        registry.register(Type::string(), "length", Type::integer());
        registry.register(Type::instance("Object"), "length", Type::string());
        let info = registry.resolve(&Type::string(), "length").unwrap();
        assert_eq!(info.return_type.base_class_name(), Some("Integer"));
    }

    // --- Types that skip fallback ---

    #[test]
    fn test_singleton_type_skips_fallback() {
        let mut registry = MethodRegistry::new();
        registry.register(Type::instance("Kernel"), "puts", Type::Nil);
        assert!(registry.resolve(&Type::singleton("User"), "puts").is_none());
    }

    #[test]
    fn test_nil_type_skips_fallback() {
        let mut registry = MethodRegistry::new();
        registry.register(Type::instance("Kernel"), "puts", Type::Nil);
        assert!(registry.resolve(&Type::Nil, "puts").is_none());
    }

    #[test]
    fn test_union_type_skips_fallback() {
        let mut registry = MethodRegistry::new();
        registry.register(Type::instance("Kernel"), "puts", Type::Nil);
        let union_ty = Type::Union(vec![Type::string(), Type::integer()]);
        assert!(registry.resolve(&union_ty, "puts").is_none());
    }

    #[test]
    fn test_bot_type_skips_fallback() {
        let mut registry = MethodRegistry::new();
        registry.register(Type::instance("Kernel"), "puts", Type::Nil);
        assert!(registry.resolve(&Type::Bot, "puts").is_none());
    }

    // --- Generic type fallback chain ---

    #[test]
    fn test_resolve_generic_falls_back_to_kernel() {
        let mut registry = MethodRegistry::new();
        registry.register(Type::instance("Kernel"), "puts", Type::Nil);
        let generic_type = Type::array_of(Type::integer());
        let info = registry.resolve(&generic_type, "puts").unwrap();
        assert_eq!(info.return_type, Type::Nil);
    }

    #[test]
    fn test_resolve_generic_full_chain() {
        // Verify the 4-step fallback: Generic[T] → Base → Object → Kernel
        let mut registry = MethodRegistry::new();
        registry.register(Type::instance("Kernel"), "object_id", Type::integer());
        let generic_type = Type::array_of(Type::string());
        // Array[String] → Array (none) → Object (none) → Kernel (exists)
        let info = registry.resolve(&generic_type, "object_id").unwrap();
        assert_eq!(info.return_type.base_class_name(), Some("Integer"));
    }

    // --- Namespaced class fallback ---

    #[test]
    fn test_resolve_namespaced_class_falls_back_to_object() {
        let mut registry = MethodRegistry::new();
        registry.register(Type::instance("Object"), "class", Type::string());
        let info = registry.resolve(&Type::instance("Api::V1::User"), "class").unwrap();
        assert_eq!(info.return_type.base_class_name(), Some("String"));
    }
}
