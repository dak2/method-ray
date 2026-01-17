use crate::env::scope::{Scope, ScopeId, ScopeKind, ScopeManager};
use crate::graph::{BoxId, BoxTrait, ChangeSet, EdgeUpdate, Source, Vertex, VertexId};
use crate::source_map::SourceLocation;
use crate::types::Type;
use std::collections::{HashMap, HashSet, VecDeque};

/// Method information
pub struct MethodInfo {
    pub return_type: Type,
}

/// Type error information for diagnostic reporting
#[derive(Debug, Clone)]
pub struct TypeError {
    pub receiver_type: Type,
    pub method_name: String,
    pub vertex_id: VertexId, // Location in the graph
    pub location: Option<SourceLocation>, // Source code location
}

/// Global environment: core of the type inference engine
pub struct GlobalEnv {
    /// Vertex management
    pub vertices: HashMap<VertexId, Vertex>,
    pub sources: HashMap<VertexId, Source>,

    /// Box management
    pub boxes: HashMap<BoxId, Box<dyn BoxTrait>>,
    pub run_queue: VecDeque<BoxId>,
    run_queue_set: HashSet<BoxId>,

    /// Method definitions
    methods: HashMap<(Type, String), MethodInfo>,

    /// Type errors collected during analysis
    pub type_errors: Vec<TypeError>,

    /// Scope management
    pub scope_manager: ScopeManager,

    /// ID generation
    next_vertex_id: usize,
    pub next_box_id: usize,
}

impl GlobalEnv {
    pub fn new() -> Self {
        Self {
            vertices: HashMap::new(),
            sources: HashMap::new(),
            boxes: HashMap::new(),
            run_queue: VecDeque::new(),
            run_queue_set: HashSet::new(),
            methods: HashMap::new(),
            type_errors: Vec::new(),
            scope_manager: ScopeManager::new(),
            next_vertex_id: 0,
            next_box_id: 0,
        }
    }

    /// Create new Vertex
    pub fn new_vertex(&mut self) -> VertexId {
        let id = VertexId(self.next_vertex_id);
        self.next_vertex_id += 1;
        self.vertices.insert(id, Vertex::new(id));
        id
    }

    /// Create new Source (fixed type)
    pub fn new_source(&mut self, ty: Type) -> VertexId {
        let id = VertexId(self.next_vertex_id);
        self.next_vertex_id += 1;
        self.sources.insert(id, Source::new(id, ty));
        id
    }

    /// Get Vertex
    pub fn get_vertex(&self, id: VertexId) -> Option<&Vertex> {
        self.vertices.get(&id)
    }

    /// Get Source
    pub fn get_source(&self, id: VertexId) -> Option<&Source> {
        self.sources.get(&id)
    }

    /// Add edge (immediate type propagation)
    pub fn add_edge(&mut self, src: VertexId, dst: VertexId) {
        // Add edge from src to dst
        if let Some(src_vtx) = self.vertices.get_mut(&src) {
            src_vtx.add_next(dst);
        }

        // Propagate type
        self.propagate_from(src, dst);
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

    /// Propagate type from src to dst
    fn propagate_from(&mut self, src: VertexId, dst: VertexId) {
        // Get src type
        let types: Vec<Type> = if let Some(src_vtx) = self.vertices.get(&src) {
            src_vtx.types.keys().cloned().collect()
        } else if let Some(src_source) = self.sources.get(&src) {
            vec![src_source.ty.clone()]
        } else {
            return;
        };

        if !types.is_empty() {
            self.propagate_types(src, dst, types);
        }
    }

    /// Recursively propagate type
    fn propagate_types(&mut self, src_id: VertexId, dst_id: VertexId, types: Vec<Type>) {
        // Add type only if dst is Vertex
        let next_propagations = if let Some(dst_vtx) = self.vertices.get_mut(&dst_id) {
            dst_vtx.on_type_added(src_id, types)
        } else {
            // If dst is Source, do nothing (fixed type)
            return;
        };

        // Recursively propagate
        for (next_id, next_types) in next_propagations {
            self.propagate_types(dst_id, next_id, next_types);
        }
    }

    /// Resolve method
    pub fn resolve_method(&self, recv_ty: &Type, method_name: &str) -> Option<&MethodInfo> {
        self.methods.get(&(recv_ty.clone(), method_name.to_string()))
    }

    /// Register built-in method
    pub fn register_builtin_method(&mut self, recv_ty: Type, method_name: &str, ret_ty: Type) {
        self.methods.insert(
            (recv_ty, method_name.to_string()),
            MethodInfo {
                return_type: ret_ty,
            },
        );
    }

    /// Record a type error (undefined method)
    pub fn record_type_error(
        &mut self,
        receiver_type: Type,
        method_name: String,
        vertex_id: VertexId,
        location: Option<SourceLocation>,
    ) {
        self.type_errors.push(TypeError {
            receiver_type,
            method_name,
            vertex_id,
            location,
        });
    }

    /// Add Box to queue
    pub fn add_run(&mut self, box_id: BoxId) {
        if !self.run_queue_set.contains(&box_id) {
            self.run_queue.push_back(box_id);
            self.run_queue_set.insert(box_id);
        }
    }

    /// Execute all Boxes
    pub fn run_all(&mut self) {
        while let Some(box_id) = self.run_queue.pop_front() {
            self.run_queue_set.remove(&box_id);

            if let Some(box_obj) = self.boxes.get_mut(&box_id) {
                let mut changes = ChangeSet::new();

                // Execute Box (temporarily remove to avoid &mut self borrow issue)
                let mut temp_box = self.boxes.remove(&box_id).unwrap();
                temp_box.run(self, &mut changes);
                self.boxes.insert(box_id, temp_box);

                self.apply_changes(changes);
            }
        }
    }

    /// For debugging: display types of all Vertices
    pub fn show_all(&self) -> String {
        let mut lines = Vec::new();

        for (id, vtx) in &self.vertices {
            lines.push(format!("Vertex {}: {}", id.0, vtx.show()));
        }

        for (id, src) in &self.sources {
            lines.push(format!("Source {}: {}", id.0, src.show()));
        }

        lines.join("\n")
    }

    // Scope-related helper methods

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

        assert_eq!(genv.get_source(s1).unwrap().show(), "String");
        assert_eq!(genv.get_source(s2).unwrap().show(), "Integer");
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
