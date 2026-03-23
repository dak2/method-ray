//! Method registration and resolution

use std::collections::{HashMap, HashSet};

use crate::graph::VertexId;
use crate::types::Type;
use smallvec::SmallVec;

const OBJECT_CLASS: &str = "Object";
const KERNEL_MODULE: &str = "Kernel";

/// Aggregated context for method resolution (inclusions, superclass chain, extensions)
pub struct ResolutionContext<'a> {
    pub inclusions: &'a HashMap<String, Vec<String>>,
    pub superclass_map: &'a HashMap<String, String>,
    pub extensions: &'a HashMap<String, Vec<String>>,
}

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

    /// Add included modules for a given class name to the chain.
    fn add_included_modules(
        chain: &mut SmallVec<[Type; 8]>,
        class_name: &str,
        inclusions: &HashMap<String, Vec<String>>,
    ) {
        if let Some(modules) = inclusions.get(class_name) {
            for module_name in modules.iter().rev() {
                chain.push(Type::instance(module_name));
            }
        }
    }

    /// Build the method resolution order (MRO) fallback chain for a receiver type.
    ///
    /// Returns a list of types to search in order:
    /// 1. Exact receiver type
    /// 2. Generic → base class (e.g., Array[Integer] → Array)
    /// 3. Included modules of self (last included first, matching Ruby MRO)
    /// 4. Superclass chain: for each parent, add parent type + its included modules
    /// 5. Object (for Instance/Generic types only)
    /// 6. Kernel (for Instance/Generic types only)
    /// 7. Extended modules (for Singleton types only, last extended has highest priority)
    fn fallback_chain(
        recv_ty: &Type,
        ctx: &ResolutionContext,
    ) -> SmallVec<[Type; 8]> {
        let mut chain = SmallVec::new();
        chain.push(recv_ty.clone());

        if let Type::Generic { name, .. } = recv_ty {
            chain.push(Type::Instance { name: name.clone() });
        }

        // MRO for Instance/Generic: included modules → superclass chain → Object → Kernel
        if matches!(recv_ty, Type::Instance { .. } | Type::Generic { .. }) {
            // Included modules of self (reverse order = last included has highest priority)
            if let Some(class_name) = recv_ty.base_class_name() {
                Self::add_included_modules(&mut chain, class_name, ctx.inclusions);

                // Walk superclass chain
                let mut visited = HashSet::new();
                visited.insert(class_name.to_string());
                let mut current = class_name.to_string();
                while let Some(parent) = ctx.superclass_map.get(&current) {
                    if !visited.insert(parent.clone()) {
                        // Cycle detected, stop walking
                        break;
                    }
                    chain.push(Type::instance(parent));
                    Self::add_included_modules(&mut chain, parent, ctx.inclusions);
                    current = parent.clone();
                }
            }

            chain.push(Type::instance(OBJECT_CLASS));
            chain.push(Type::instance(KERNEL_MODULE));
        }

        // Singleton type: search extended modules (extend makes module methods available as class methods)
        if let Type::Singleton { name } = recv_ty {
            Self::add_included_modules(&mut chain, name.full_name(), ctx.extensions);
        }

        chain
    }

    /// Resolve a method for a receiver type
    ///
    /// Searches the MRO fallback chain: exact type → base class (for generics)
    /// → included modules → superclass chain → Object → Kernel.
    /// For Singleton types, also searches extended modules after exact match.
    /// For other non-instance types (Nil, Union, Bot), only exact match is attempted.
    pub fn resolve(
        &self,
        recv_ty: &Type,
        method_name: &str,
        ctx: &ResolutionContext,
    ) -> Option<&MethodInfo> {
        let method_key = method_name.to_string();
        Self::fallback_chain(recv_ty, ctx)
            .into_iter()
            .find_map(|ty| self.methods.get(&(ty, method_key.clone())))
    }
}
