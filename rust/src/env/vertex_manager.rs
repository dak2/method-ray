//! Vertex and Source management
//!
//! Handles creation, storage, and type propagation for vertices and sources.

use crate::graph::{Source, Vertex, VertexId};
use crate::types::Type;
use std::collections::HashMap;

/// Manages vertices and sources in the type graph
#[derive(Debug, Default)]
pub struct VertexManager {
    /// All vertices in the graph
    pub vertices: HashMap<VertexId, Vertex>,
    /// All sources (fixed-type nodes) in the graph
    pub sources: HashMap<VertexId, Source>,
    /// Next vertex ID to allocate
    next_vertex_id: usize,
}

#[allow(dead_code)]
impl VertexManager {
    /// Create a new empty vertex manager
    pub fn new() -> Self {
        Self {
            vertices: HashMap::new(),
            sources: HashMap::new(),
            next_vertex_id: 0,
        }
    }

    /// Create a new vertex and return its ID
    pub fn new_vertex(&mut self) -> VertexId {
        let id = VertexId(self.next_vertex_id);
        self.next_vertex_id += 1;
        self.vertices.insert(id, Vertex::new());
        id
    }

    /// Create a new source with a fixed type
    pub fn new_source(&mut self, ty: Type) -> VertexId {
        let id = VertexId(self.next_vertex_id);
        self.next_vertex_id += 1;
        self.sources.insert(id, Source::new(ty));
        id
    }

    /// Get a vertex by ID
    pub fn get_vertex(&self, id: VertexId) -> Option<&Vertex> {
        self.vertices.get(&id)
    }

    /// Get a vertex mutably by ID
    pub fn get_vertex_mut(&mut self, id: VertexId) -> Option<&mut Vertex> {
        self.vertices.get_mut(&id)
    }

    /// Get a source by ID
    pub fn get_source(&self, id: VertexId) -> Option<&Source> {
        self.sources.get(&id)
    }

    /// Add an edge between two vertices and propagate types
    pub fn add_edge(&mut self, src: VertexId, dst: VertexId) {
        // Add edge from src to dst
        if let Some(src_vtx) = self.vertices.get_mut(&src) {
            src_vtx.add_next(dst);
        }

        // Propagate type
        self.propagate_from(src, dst);
    }

    /// Get types from a vertex or source
    fn get_types(&self, id: VertexId) -> Vec<Type> {
        if let Some(vtx) = self.vertices.get(&id) {
            vtx.types.keys().cloned().collect()
        } else if let Some(src) = self.sources.get(&id) {
            vec![src.ty.clone()]
        } else {
            vec![]
        }
    }

    /// Propagate types from src to dst
    fn propagate_from(&mut self, src: VertexId, dst: VertexId) {
        let types = self.get_types(src);
        if !types.is_empty() {
            self.propagate_types(src, dst, types);
        }
    }

    /// Recursively propagate types through the graph
    fn propagate_types(&mut self, src_id: VertexId, dst_id: VertexId, types: Vec<Type>) {
        // Add type only if dst is a Vertex (not a Source)
        let next_propagations = if let Some(dst_vtx) = self.vertices.get_mut(&dst_id) {
            dst_vtx.on_type_added(src_id, types)
        } else {
            // If dst is a Source, do nothing (fixed type)
            return;
        };

        // Recursively propagate to next vertices
        for (next_id, next_types) in next_propagations {
            self.propagate_types(dst_id, next_id, next_types);
        }
    }

    /// Display all vertices and sources for debugging
    pub fn show_all(&self) -> String {
        let mut lines = Vec::new();

        for (id, vtx) in &self.vertices {
            lines.push(format!("Vertex {}: {}", id.0, vtx.show()));
        }

        for (id, src) in &self.sources {
            lines.push(format!("Source {}: {}", id.0, src.ty.show()));
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_vertex() {
        let mut manager = VertexManager::new();

        let v1 = manager.new_vertex();
        let v2 = manager.new_vertex();

        assert_eq!(v1.0, 0);
        assert_eq!(v2.0, 1);
        assert!(manager.get_vertex(v1).is_some());
        assert!(manager.get_vertex(v2).is_some());
    }

    #[test]
    fn test_new_source() {
        let mut manager = VertexManager::new();

        let s1 = manager.new_source(Type::string());
        let s2 = manager.new_source(Type::integer());

        assert_eq!(manager.get_source(s1).unwrap().ty.show(), "String");
        assert_eq!(manager.get_source(s2).unwrap().ty.show(), "Integer");
    }

    #[test]
    fn test_edge_propagation() {
        let mut manager = VertexManager::new();

        let src = manager.new_source(Type::string());
        let vtx = manager.new_vertex();

        manager.add_edge(src, vtx);

        assert_eq!(manager.get_vertex(vtx).unwrap().show(), "String");
    }

    #[test]
    fn test_chain_propagation() {
        let mut manager = VertexManager::new();

        let src = manager.new_source(Type::string());
        let v1 = manager.new_vertex();
        let v2 = manager.new_vertex();

        manager.add_edge(src, v1);
        manager.add_edge(v1, v2);

        assert_eq!(manager.get_vertex(v1).unwrap().show(), "String");
        assert_eq!(manager.get_vertex(v2).unwrap().show(), "String");
    }

    #[test]
    fn test_union_propagation() {
        let mut manager = VertexManager::new();

        let src1 = manager.new_source(Type::string());
        let src2 = manager.new_source(Type::integer());
        let vtx = manager.new_vertex();

        manager.add_edge(src1, vtx);
        manager.add_edge(src2, vtx);

        assert_eq!(
            manager.get_vertex(vtx).unwrap().show(),
            "(Integer | String)"
        );
    }
}
