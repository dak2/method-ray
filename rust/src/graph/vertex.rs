use crate::types::Type;
use std::collections::{HashMap, HashSet};

/// Vertex ID (uniquely identifies a vertex in the graph)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VertexId(pub usize);

/// Source: Vertex with fixed type (e.g., literals)
#[derive(Debug, Clone)]
pub struct Source {
    pub ty: Type,
}

impl Source {
    pub fn new(ty: Type) -> Self {
        Self { ty }
    }
}

/// Vertex: Vertex that dynamically accumulates types (e.g., variables)
#[derive(Debug, Clone)]
pub struct Vertex {
    /// Type -> Sources (set of Source IDs that provided this type)
    pub types: HashMap<Type, HashSet<VertexId>>,
    /// Set of connected Vertex IDs
    pub next_vtxs: HashSet<VertexId>,
}

impl Vertex {
    pub fn new() -> Self {
        Self {
            types: HashMap::new(),
            next_vtxs: HashSet::new(),
        }
    }

    /// Add connection destination
    pub fn add_next(&mut self, next_id: VertexId) {
        self.next_vtxs.insert(next_id);
    }

    /// Add type (core of type propagation)
    /// Returns: list of newly added types and destinations to propagate to
    pub fn on_type_added(&mut self, src_id: VertexId, added_types: Vec<Type>) -> Vec<(VertexId, Vec<Type>)> {
        let mut new_added_types = Vec::new();

        for ty in added_types {
            if let Some(sources) = self.types.get_mut(&ty) {
                // Type already exists: add Source
                sources.insert(src_id);
            } else {
                // New type: add type and record Source
                let mut sources = HashSet::new();
                sources.insert(src_id);
                self.types.insert(ty.clone(), sources);
                new_added_types.push(ty);
            }
        }

        // If no new types, don't propagate anything
        if new_added_types.is_empty() {
            return vec![];
        }

        // Propagate to connections
        self.next_vtxs
            .iter()
            .map(|&next_id| (next_id, new_added_types.clone()))
            .collect()
    }

    /// Convert type to string representation
    pub fn show(&self) -> String {
        if self.types.is_empty() {
            return "untyped".to_string();
        }

        let mut type_strs: Vec<_> = self.types.keys().map(|t| t.show()).collect();
        type_strs.sort();
        type_strs.dedup();

        if type_strs.len() == 1 {
            type_strs[0].clone()
        } else {
            format!("({})", type_strs.join(" | "))
        }
    }
}

impl Default for Vertex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_type() {
        let src = Source::new(Type::string());
        assert_eq!(src.ty.show(), "String");
    }

    #[test]
    fn test_vertex_empty() {
        let vtx = Vertex::new();
        assert_eq!(vtx.show(), "untyped");
    }

    #[test]
    fn test_vertex_single_type() {
        let mut vtx = Vertex::new();

        // Add String type
        let propagations = vtx.on_type_added(VertexId(1), vec![Type::string()]);
        assert_eq!(vtx.show(), "String");
        assert_eq!(propagations.len(), 0); // No propagation since no connections
    }

    #[test]
    fn test_vertex_union_type() {
        let mut vtx = Vertex::new();

        // Add String type
        vtx.on_type_added(VertexId(1), vec![Type::string()]);
        assert_eq!(vtx.show(), "String");

        // Add Integer type → becomes Union type
        vtx.on_type_added(VertexId(1), vec![Type::integer()]);
        assert_eq!(vtx.show(), "(Integer | String)");
    }

    #[test]
    fn test_vertex_propagation() {
        let mut vtx = Vertex::new();

        // Add connections
        vtx.add_next(VertexId(3));
        vtx.add_next(VertexId(4));

        // Add type → propagated to connections
        let propagations = vtx.on_type_added(VertexId(1), vec![Type::string()]);

        assert_eq!(propagations.len(), 2);
        assert!(propagations.contains(&(VertexId(3), vec![Type::string()])));
        assert!(propagations.contains(&(VertexId(4), vec![Type::string()])));
    }

    #[test]
    fn test_vertex_no_duplicate_propagation() {
        let mut vtx = Vertex::new();
        vtx.add_next(VertexId(3));

        // Add same type twice → only first time propagates
        let prop1 = vtx.on_type_added(VertexId(1), vec![Type::string()]);
        assert_eq!(prop1.len(), 1);

        let prop2 = vtx.on_type_added(VertexId(1), vec![Type::string()]);
        assert_eq!(prop2.len(), 0); // No propagation since already exists
    }
}
