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
    /// 3. Included modules (last included first, matching Ruby MRO)
    /// 4. Object (for Instance/Generic types only)
    /// 5. Kernel (for Instance/Generic types only)
    fn fallback_chain(
        recv_ty: &Type,
        inclusions: &HashMap<String, Vec<String>>,
    ) -> SmallVec<[Type; 8]> {
        let mut chain = SmallVec::new();
        chain.push(recv_ty.clone());

        if let Type::Generic { name, .. } = recv_ty {
            chain.push(Type::Instance { name: name.clone() });
        }

        // MRO for Instance/Generic: included modules → Object → Kernel
        if matches!(recv_ty, Type::Instance { .. } | Type::Generic { .. }) {
            // Included modules (reverse order = last included has highest priority)
            if let Some(class_name) = recv_ty.base_class_name() {
                if let Some(modules) = inclusions.get(class_name) {
                    for module_name in modules.iter().rev() {
                        chain.push(Type::instance(module_name));
                    }
                }
            }

            chain.push(Type::instance(OBJECT_CLASS));
            chain.push(Type::instance(KERNEL_MODULE));
        }

        chain
    }

    /// Resolve a method for a receiver type
    ///
    /// Searches the MRO fallback chain: exact type → base class (for generics)
    /// → included modules → Object → Kernel.
    /// For non-instance types (Singleton, Nil, Union, Bot), only exact match is attempted.
    pub fn resolve(
        &self,
        recv_ty: &Type,
        method_name: &str,
        inclusions: &HashMap<String, Vec<String>>,
    ) -> Option<&MethodInfo> {
        let method_key = method_name.to_string();
        Self::fallback_chain(recv_ty, inclusions)
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

        let info = registry.resolve(&Type::string(), "length", &HashMap::new()).unwrap();
        assert_eq!(info.return_type.base_class_name(), Some("Integer"));
    }

    #[test]
    fn test_resolve_not_found() {
        let registry = MethodRegistry::new();
        assert!(registry.resolve(&Type::string(), "unknown", &HashMap::new()).is_none());
    }

    #[test]
    fn test_register_user_method_and_resolve() {
        let mut registry = MethodRegistry::new();
        let return_vtx = VertexId(42);
        registry.register_user_method(Type::instance("User"), "name", return_vtx, vec![], None);

        let info = registry.resolve(&Type::instance("User"), "name", &HashMap::new()).unwrap();
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

        let info = registry.resolve(&Type::instance("Calc"), "add", &HashMap::new()).unwrap();
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
        let info = registry.resolve(&Type::instance("CustomClass"), "nil?", &HashMap::new()).unwrap();
        assert_eq!(info.return_type.base_class_name(), Some("TrueClass"));
    }

    #[test]
    fn test_resolve_falls_back_to_kernel() {
        let mut registry = MethodRegistry::new();
        registry.register(Type::instance("Kernel"), "puts", Type::Nil);
        let info = registry.resolve(&Type::instance("MyApp"), "puts", &HashMap::new()).unwrap();
        assert_eq!(info.return_type, Type::Nil);
    }

    #[test]
    fn test_resolve_object_before_kernel() {
        let mut registry = MethodRegistry::new();
        registry.register(Type::instance("Object"), "to_s", Type::string());
        registry.register(Type::instance("Kernel"), "to_s", Type::integer());
        let info = registry.resolve(&Type::instance("Anything"), "to_s", &HashMap::new()).unwrap();
        assert_eq!(info.return_type.base_class_name(), Some("String"));
    }

    #[test]
    fn test_resolve_exact_match_over_fallback() {
        let mut registry = MethodRegistry::new();
        registry.register(Type::string(), "length", Type::integer());
        registry.register(Type::instance("Object"), "length", Type::string());
        let info = registry.resolve(&Type::string(), "length", &HashMap::new()).unwrap();
        assert_eq!(info.return_type.base_class_name(), Some("Integer"));
    }

    // --- Types that skip fallback ---

    #[test]
    fn test_singleton_type_skips_fallback() {
        let mut registry = MethodRegistry::new();
        registry.register(Type::instance("Kernel"), "puts", Type::Nil);
        assert!(registry.resolve(&Type::singleton("User"), "puts", &HashMap::new()).is_none());
    }

    #[test]
    fn test_nil_type_skips_fallback() {
        let mut registry = MethodRegistry::new();
        registry.register(Type::instance("Kernel"), "puts", Type::Nil);
        assert!(registry.resolve(&Type::Nil, "puts", &HashMap::new()).is_none());
    }

    #[test]
    fn test_union_type_skips_fallback() {
        let mut registry = MethodRegistry::new();
        registry.register(Type::instance("Kernel"), "puts", Type::Nil);
        let union_ty = Type::Union(vec![Type::string(), Type::integer()]);
        assert!(registry.resolve(&union_ty, "puts", &HashMap::new()).is_none());
    }

    #[test]
    fn test_bot_type_skips_fallback() {
        let mut registry = MethodRegistry::new();
        registry.register(Type::instance("Kernel"), "puts", Type::Nil);
        assert!(registry.resolve(&Type::Bot, "puts", &HashMap::new()).is_none());
    }

    // --- Generic type fallback chain ---

    #[test]
    fn test_resolve_generic_falls_back_to_kernel() {
        let mut registry = MethodRegistry::new();
        registry.register(Type::instance("Kernel"), "puts", Type::Nil);
        let generic_type = Type::array_of(Type::integer());
        let info = registry.resolve(&generic_type, "puts", &HashMap::new()).unwrap();
        assert_eq!(info.return_type, Type::Nil);
    }

    #[test]
    fn test_resolve_generic_full_chain() {
        // Verify the 4-step fallback: Generic[T] → Base → Object → Kernel
        let mut registry = MethodRegistry::new();
        registry.register(Type::instance("Kernel"), "object_id", Type::integer());
        let generic_type = Type::array_of(Type::string());
        // Array[String] → Array (none) → Object (none) → Kernel (exists)
        let info = registry.resolve(&generic_type, "object_id", &HashMap::new()).unwrap();
        assert_eq!(info.return_type.base_class_name(), Some("Integer"));
    }

    // --- Namespaced class fallback ---

    #[test]
    fn test_resolve_namespaced_class_falls_back_to_object() {
        let mut registry = MethodRegistry::new();
        registry.register(Type::instance("Object"), "class", Type::string());
        let info = registry.resolve(&Type::instance("Api::V1::User"), "class", &HashMap::new()).unwrap();
        assert_eq!(info.return_type.base_class_name(), Some("String"));
    }

    // --- Include (mixin) fallback ---

    #[test]
    fn test_resolve_with_include() {
        let mut registry = MethodRegistry::new();
        registry.register(Type::instance("Greetable"), "greet", Type::string());

        let mut inclusions = HashMap::new();
        inclusions.insert("User".to_string(), vec!["Greetable".to_string()]);

        let info = registry.resolve(&Type::instance("User"), "greet", &inclusions).unwrap();
        assert_eq!(info.return_type.base_class_name(), Some("String"));
    }

    #[test]
    fn test_resolve_include_order() {
        // include A; include B → B's method found first (MRO: last included wins)
        let mut registry = MethodRegistry::new();
        registry.register(Type::instance("A"), "foo", Type::string());
        registry.register(Type::instance("B"), "foo", Type::integer());

        let mut inclusions = HashMap::new();
        inclusions.insert("User".to_string(), vec!["A".to_string(), "B".to_string()]);

        let info = registry.resolve(&Type::instance("User"), "foo", &inclusions).unwrap();
        assert_eq!(info.return_type.base_class_name(), Some("Integer"));
    }

    #[test]
    fn test_resolve_class_method_over_include() {
        // Class's own method takes priority over included module
        let mut registry = MethodRegistry::new();
        registry.register(Type::instance("Greetable"), "greet", Type::string());
        registry.register(Type::instance("User"), "greet", Type::integer());

        let mut inclusions = HashMap::new();
        inclusions.insert("User".to_string(), vec!["Greetable".to_string()]);

        let info = registry.resolve(&Type::instance("User"), "greet", &inclusions).unwrap();
        assert_eq!(info.return_type.base_class_name(), Some("Integer"));
    }

    #[test]
    fn test_resolve_include_before_object() {
        // Included module is searched before Object in MRO
        let mut registry = MethodRegistry::new();
        registry.register(Type::instance("Object"), "foo", Type::string());
        registry.register(Type::instance("MyModule"), "foo", Type::integer());

        let mut inclusions = HashMap::new();
        inclusions.insert("User".to_string(), vec!["MyModule".to_string()]);

        let info = registry.resolve(&Type::instance("User"), "foo", &inclusions).unwrap();
        assert_eq!(info.return_type.base_class_name(), Some("Integer"));
    }

    #[test]
    fn test_singleton_type_skips_include_fallback() {
        // include adds instance methods only — Singleton (class-level) should NOT resolve
        let mut registry = MethodRegistry::new();
        registry.register(Type::instance("Greetable"), "greet", Type::string());

        let mut inclusions = HashMap::new();
        inclusions.insert("User".to_string(), vec!["Greetable".to_string()]);

        assert!(registry.resolve(&Type::singleton("User"), "greet", &inclusions).is_none());
    }
}
