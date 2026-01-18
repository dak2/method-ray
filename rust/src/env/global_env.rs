//! Global environment: facade for the type inference engine
//!
//! This module provides a unified interface for managing vertices, boxes,
//! methods, type errors, and scopes during type inference.

use crate::env::box_manager::BoxManager;
use crate::env::method_registry::{MethodInfo, MethodRegistry};
use crate::env::scope::{Scope, ScopeId, ScopeKind, ScopeManager};
use crate::env::type_error::TypeError;
use crate::env::vertex_manager::VertexManager;
use crate::graph::{BoxId, BoxTrait, ChangeSet, EdgeUpdate, Source, Vertex, VertexId};
use crate::source_map::SourceLocation;
use crate::types::Type;

/// Global environment: core of the type inference engine
///
/// This is a facade that coordinates the various subsystems:
/// - Vertex management (type graph nodes)
/// - Box management (reactive computations)
/// - Method registry (method definitions)
/// - Type errors (diagnostic collection)
/// - Scope management (lexical scopes)
pub struct GlobalEnv {
    /// Vertex and source management
    vertex_manager: VertexManager,

    /// Box management and execution queue
    box_manager: BoxManager,

    /// Method definitions
    method_registry: MethodRegistry,

    /// Type errors collected during analysis
    pub type_errors: Vec<TypeError>,

    /// Scope management
    pub scope_manager: ScopeManager,
}

#[allow(dead_code)]
impl GlobalEnv {
    pub fn new() -> Self {
        Self {
            vertex_manager: VertexManager::new(),
            box_manager: BoxManager::new(),
            method_registry: MethodRegistry::new(),
            type_errors: Vec::new(),
            scope_manager: ScopeManager::new(),
        }
    }

    // ===== Vertex Management =====

    /// Create new Vertex
    pub fn new_vertex(&mut self) -> VertexId {
        self.vertex_manager.new_vertex()
    }

    /// Create new Source (fixed type)
    pub fn new_source(&mut self, ty: Type) -> VertexId {
        self.vertex_manager.new_source(ty)
    }

    /// Get Vertex
    pub fn get_vertex(&self, id: VertexId) -> Option<&Vertex> {
        self.vertex_manager.get_vertex(id)
    }

    /// Get Source
    pub fn get_source(&self, id: VertexId) -> Option<&Source> {
        self.vertex_manager.get_source(id)
    }

    /// Add edge (immediate type propagation)
    pub fn add_edge(&mut self, src: VertexId, dst: VertexId) {
        self.vertex_manager.add_edge(src, dst);
    }

    /// For debugging: display types of all Vertices
    pub fn show_all(&self) -> String {
        self.vertex_manager.show_all()
    }

    // ===== Box Management =====

    /// Allocate a new BoxId
    pub fn alloc_box_id(&mut self) -> BoxId {
        let id = BoxId(self.box_manager.next_box_id);
        self.box_manager.next_box_id += 1;
        id
    }

    /// Register a Box with a pre-allocated ID and add it to the run queue
    pub fn register_box(&mut self, box_id: BoxId, box_instance: Box<dyn BoxTrait>) {
        self.box_manager.insert(box_id, box_instance);
        self.box_manager.add_run(box_id);
    }

    /// Get the number of registered boxes
    pub fn box_count(&self) -> usize {
        self.box_manager.len()
    }

    /// Apply changes
    pub fn apply_changes(&mut self, mut changes: ChangeSet) {
        let updates = changes.reinstall();

        for update in updates {
            match update {
                EdgeUpdate::Add { src, dst } => {
                    self.add_edge(src, dst);
                }
                EdgeUpdate::Remove { .. } => {
                    // TODO: Implement edge removal (in Phase 2)
                }
            }
        }
    }

    /// Execute all Boxes
    pub fn run_all(&mut self) {
        while let Some(box_id) = self.box_manager.pop_run() {
            if self.box_manager.contains(box_id) {
                let mut changes = ChangeSet::new();

                // Execute Box (temporarily remove to avoid &mut self borrow issue)
                let mut temp_box = self.box_manager.remove(box_id).unwrap();
                temp_box.run(self, &mut changes);
                self.box_manager.insert(box_id, temp_box);

                self.apply_changes(changes);
            }
        }
    }

    // ===== Method Registry =====

    /// Resolve method
    pub fn resolve_method(&self, recv_ty: &Type, method_name: &str) -> Option<&MethodInfo> {
        self.method_registry.resolve(recv_ty, method_name)
    }

    /// Register built-in method
    pub fn register_builtin_method(&mut self, recv_ty: Type, method_name: &str, ret_ty: Type) {
        self.method_registry.register(recv_ty, method_name, ret_ty);
    }

    // ===== Type Errors =====

    /// Record a type error (undefined method)
    pub fn record_type_error(
        &mut self,
        receiver_type: Type,
        method_name: String,
        location: Option<SourceLocation>,
    ) {
        self.type_errors
            .push(TypeError::new(receiver_type, method_name, location));
    }

    // ===== Scope Management =====

    /// Enter a class scope
    pub fn enter_class(&mut self, name: String) -> ScopeId {
        let scope_id = self.scope_manager.new_scope(ScopeKind::Class {
            name,
            superclass: None,
        });
        self.scope_manager.enter_scope(scope_id);
        scope_id
    }

    /// Enter a method scope
    pub fn enter_method(&mut self, name: String) -> ScopeId {
        let class_name = self.scope_manager.current_class_name();
        let scope_id = self.scope_manager.new_scope(ScopeKind::Method {
            name,
            receiver_type: class_name,
        });
        self.scope_manager.enter_scope(scope_id);
        scope_id
    }

    /// Exit current scope
    pub fn exit_scope(&mut self) {
        self.scope_manager.exit_scope();
    }

    /// Get current scope
    pub fn current_scope(&self) -> &Scope {
        self.scope_manager.current_scope()
    }

    /// Get current scope mutably
    pub fn current_scope_mut(&mut self) -> &mut Scope {
        self.scope_manager.current_scope_mut()
    }
}

impl Default for GlobalEnv {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_env_new_vertex() {
        let mut genv = GlobalEnv::new();

        let v1 = genv.new_vertex();
        let v2 = genv.new_vertex();

        assert_eq!(v1.0, 0);
        assert_eq!(v2.0, 1);
    }

    #[test]
    fn test_global_env_new_source() {
        let mut genv = GlobalEnv::new();

        let s1 = genv.new_source(Type::string());
        let s2 = genv.new_source(Type::integer());

        assert_eq!(genv.get_source(s1).unwrap().ty.show(), "String");
        assert_eq!(genv.get_source(s2).unwrap().ty.show(), "Integer");
    }

    #[test]
    fn test_global_env_edge_propagation() {
        let mut genv = GlobalEnv::new();

        // Source<String> -> Vertex
        let src = genv.new_source(Type::string());
        let vtx = genv.new_vertex();

        genv.add_edge(src, vtx);

        // Verify type propagated to Vertex
        assert_eq!(genv.get_vertex(vtx).unwrap().show(), "String");
    }

    #[test]
    fn test_global_env_chain_propagation() {
        let mut genv = GlobalEnv::new();

        // Source<String> -> Vertex1 -> Vertex2
        let src = genv.new_source(Type::string());
        let v1 = genv.new_vertex();
        let v2 = genv.new_vertex();

        genv.add_edge(src, v1);
        genv.add_edge(v1, v2);

        // Verify type propagated to v2
        assert_eq!(genv.get_vertex(v1).unwrap().show(), "String");
        assert_eq!(genv.get_vertex(v2).unwrap().show(), "String");
    }

    #[test]
    fn test_global_env_union_propagation() {
        let mut genv = GlobalEnv::new();

        // Source<String> -> Vertex
        // Source<Integer> -> Vertex
        let src1 = genv.new_source(Type::string());
        let src2 = genv.new_source(Type::integer());
        let vtx = genv.new_vertex();

        genv.add_edge(src1, vtx);
        genv.add_edge(src2, vtx);

        // Verify it became Union type
        assert_eq!(genv.get_vertex(vtx).unwrap().show(), "(Integer | String)");
    }
}
