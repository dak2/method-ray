use crate::env::GlobalEnv;
use crate::graph::change_set::ChangeSet;
use crate::graph::vertex::VertexId;
use crate::source_map::SourceLocation;
use crate::types::Type;

/// Unique ID for Box
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BoxId(pub usize);

/// Box trait: represents constraints such as method calls
#[allow(dead_code)]
pub trait BoxTrait: Send + Sync {
    fn id(&self) -> BoxId;
    fn run(&mut self, genv: &mut GlobalEnv, changes: &mut ChangeSet);
    fn ret(&self) -> VertexId;
}

/// Box representing a method call
#[allow(dead_code)]
pub struct MethodCallBox {
    id: BoxId,
    recv: VertexId,
    method_name: String,
    ret: VertexId,
    location: Option<SourceLocation>, // Source code location
}

impl MethodCallBox {
    pub fn new(
        id: BoxId,
        recv: VertexId,
        method_name: String,
        ret: VertexId,
        location: Option<SourceLocation>,
    ) -> Self {
        Self {
            id,
            recv,
            method_name,
            ret,
            location,
        }
    }
}

impl BoxTrait for MethodCallBox {
    fn id(&self) -> BoxId {
        self.id
    }

    fn ret(&self) -> VertexId {
        self.ret
    }

    fn run(&mut self, genv: &mut GlobalEnv, changes: &mut ChangeSet) {
        // Get receiver type (handles both Vertex and Source)
        let recv_types: Vec<Type> = if let Some(recv_vertex) = genv.get_vertex(self.recv) {
            // Vertex case: may have multiple types
            recv_vertex.types.keys().cloned().collect()
        } else if let Some(recv_source) = genv.get_source(self.recv) {
            // Source case: has one fixed type (e.g., literals)
            vec![recv_source.ty.clone()]
        } else {
            // Receiver not found
            return;
        };

        for recv_ty in recv_types {
            // Resolve method
            if let Some(method_info) = genv.resolve_method(&recv_ty, &self.method_name) {
                // Create return type as Source
                let ret_src_id = genv.new_source(method_info.return_type.clone());

                // Add edge to return value
                changes.add_edge(ret_src_id, self.ret);
            } else {
                // Record type error for diagnostic reporting
                genv.record_type_error(
                    recv_ty.clone(),
                    self.method_name.clone(),
                    self.location.clone(),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::GlobalEnv;
    use crate::types::Type;

    #[test]
    fn test_method_call_box() {
        let mut genv = GlobalEnv::new();

        // Register String#upcase
        genv.register_builtin_method(
            Type::string(),
            "upcase",
            Type::string(),
        );

        // x = "hello" (Source<String> -> Vertex)
        let x_vtx = genv.new_vertex();
        let str_src = genv.new_source(Type::string());
        genv.add_edge(str_src, x_vtx);

        // x.upcase
        let ret_vtx = genv.new_vertex();
        let box_id = BoxId(0);
        let mut call_box = MethodCallBox::new(
            box_id,
            x_vtx,
            "upcase".to_string(),
            ret_vtx,
            None, // No location in test
        );

        // Execute Box
        let mut changes = ChangeSet::new();
        call_box.run(&mut genv, &mut changes);
        genv.apply_changes(changes);

        // Check return type
        let ret_vertex = genv.get_vertex(ret_vtx).unwrap();
        assert_eq!(ret_vertex.show(), "String");
    }

    #[test]
    fn test_method_call_box_undefined() {
        let mut genv = GlobalEnv::new();

        // Don't register method

        let x_vtx = genv.new_vertex();
        let str_src = genv.new_source(Type::string());
        genv.add_edge(str_src, x_vtx);

        // x.unknown_method (undefined)
        let ret_vtx = genv.new_vertex();
        let box_id = BoxId(0);
        let mut call_box = MethodCallBox::new(
            box_id,
            x_vtx,
            "unknown_method".to_string(),
            ret_vtx,
            None, // No location in test
        );

        let mut changes = ChangeSet::new();
        call_box.run(&mut genv, &mut changes);
        genv.apply_changes(changes);

        // Return value is untyped
        let ret_vertex = genv.get_vertex(ret_vtx).unwrap();
        assert_eq!(ret_vertex.show(), "untyped");
    }
}
