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
    /// Number of times this box has been rescheduled
    reschedule_count: u8,
}

/// Maximum number of reschedules before giving up
const MAX_RESCHEDULE_COUNT: u8 = 3;

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
            reschedule_count: 0,
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

        // If receiver has no types yet, reschedule this box for later
        // This handles cases like block parameters that are typed later
        if recv_types.is_empty() {
            if self.reschedule_count < MAX_RESCHEDULE_COUNT {
                self.reschedule_count += 1;
                changes.reschedule(self.id);
            }
            // If max reschedules reached, just skip (receiver type is unknown)
            return;
        }

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

/// Box for resolving block parameter types from method call receiver
///
/// When a method with a block is called (e.g., `str.each_char { |c| ... }`),
/// this box resolves the block parameter types from the method's RBS definition
/// and propagates them to the block parameter vertices.
#[allow(dead_code)]
pub struct BlockParameterTypeBox {
    id: BoxId,
    /// Receiver vertex of the method call
    recv_vtx: VertexId,
    /// Method name being called
    method_name: String,
    /// Block parameter vertices (in order)
    block_param_vtxs: Vec<VertexId>,
}

impl BlockParameterTypeBox {
    pub fn new(
        id: BoxId,
        recv_vtx: VertexId,
        method_name: String,
        block_param_vtxs: Vec<VertexId>,
    ) -> Self {
        Self {
            id,
            recv_vtx,
            method_name,
            block_param_vtxs,
        }
    }

    /// Check if a type is a type variable name (e.g., Elem, K, V)
    fn is_type_variable_name(name: &str) -> bool {
        matches!(
            name,
            "Elem" | "K" | "V" | "T" | "U" | "A" | "B" | "Element" | "Key" | "Value" | "Out" | "In"
        )
    }

    /// Try to resolve a type variable from receiver's type arguments.
    ///
    /// For `Array[Integer]#each { |x| }`, the block param type is `Elem`.
    /// This resolves `Elem` → `Integer` using Array's type argument.
    ///
    /// Type variable mapping for common generic classes:
    /// - Array[Elem]: Elem → type_args[0]
    /// - Hash[K, V]: K → type_args[0], V → type_args[1]
    fn resolve_type_variable(ty: &Type, recv_ty: &Type) -> Option<Type> {
        let type_var_name = match ty {
            Type::Instance { name } if Self::is_type_variable_name(name.full_name()) => {
                name.full_name()
            }
            _ => return None, // Not a type variable
        };

        // Get type arguments from receiver
        let type_args = recv_ty.type_args()?;
        let class_name = recv_ty.base_class_name()?;

        // Map type variable to type argument index based on class
        let index = match (class_name, type_var_name) {
            // Array[Elem]
            ("Array", "Elem") => 0,
            ("Array", "T") => 0,
            ("Array", "Element") => 0,
            // Hash[K, V]
            ("Hash", "K") | ("Hash", "Key") => 0,
            ("Hash", "V") | ("Hash", "Value") => 1,
            // Generic fallback: first type arg for common names
            (_, "Elem") | (_, "T") | (_, "Element") => 0,
            _ => return None,
        };

        type_args.get(index).cloned()
    }
}

impl BoxTrait for BlockParameterTypeBox {
    fn id(&self) -> BoxId {
        self.id
    }

    fn ret(&self) -> VertexId {
        // This box doesn't have a single return value
        // Return first param vtx as a placeholder
        self.block_param_vtxs
            .first()
            .copied()
            .unwrap_or(VertexId(0))
    }

    fn run(&mut self, genv: &mut GlobalEnv, changes: &mut ChangeSet) {
        // Get receiver types
        let recv_types: Vec<Type> = if let Some(recv_vertex) = genv.get_vertex(self.recv_vtx) {
            recv_vertex.types.keys().cloned().collect()
        } else if let Some(recv_source) = genv.get_source(self.recv_vtx) {
            vec![recv_source.ty.clone()]
        } else {
            return;
        };

        for recv_ty in recv_types {
            // Resolve method to get block parameter types
            // Clone the block_param_types to avoid borrow issues
            let block_param_types = genv
                .resolve_method(&recv_ty, &self.method_name)
                .and_then(|info| info.block_param_types.clone());

            if let Some(param_types) = block_param_types {
                // Map block parameter types to vertices
                for (i, param_type) in param_types.iter().enumerate() {
                    if i < self.block_param_vtxs.len() {
                        let param_vtx = self.block_param_vtxs[i];

                        // Try to resolve type variable from receiver's type arguments
                        let resolved_type =
                            if let Some(resolved) = Self::resolve_type_variable(param_type, &recv_ty) {
                                // Type variable resolved (e.g., Elem → Integer)
                                resolved
                            } else if let Type::Instance { name } = &param_type {
                                if Self::is_type_variable_name(name.full_name()) {
                                    // Type variable couldn't be resolved, skip
                                    continue;
                                } else {
                                    // Regular type, use as-is
                                    param_type.clone()
                                }
                            } else {
                                // Other type (Union, Generic, etc.), use as-is
                                param_type.clone()
                            };

                        // Create source with the resolved type
                        let src_id = genv.new_source(resolved_type);
                        changes.add_edge(src_id, param_vtx);
                    }
                }
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
        genv.register_builtin_method(Type::string(), "upcase", Type::string());

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

    #[test]
    fn test_block_param_type_box_simple() {
        let mut genv = GlobalEnv::new();

        // Register String#each_char with block param type String
        genv.register_builtin_method_with_block(
            Type::string(),
            "each_char",
            Type::string(),
            Some(vec![Type::string()]),
        );

        // Create receiver vertex with String type
        let recv_vtx = genv.new_vertex();
        let str_src = genv.new_source(Type::string());
        genv.add_edge(str_src, recv_vtx);

        // Create block parameter vertex
        let param_vtx = genv.new_vertex();

        // Create and run BlockParameterTypeBox
        let box_id = genv.alloc_box_id();
        let block_box = BlockParameterTypeBox::new(
            box_id,
            recv_vtx,
            "each_char".to_string(),
            vec![param_vtx],
        );
        genv.register_box(box_id, Box::new(block_box));

        // Run all boxes
        genv.run_all();

        // Block parameter should now have String type
        assert_eq!(genv.get_vertex(param_vtx).unwrap().show(), "String");
    }

    #[test]
    fn test_block_param_type_variable_skipped() {
        let mut genv = GlobalEnv::new();

        // Register Array#each with block param type Elem (type variable)
        genv.register_builtin_method_with_block(
            Type::array(),
            "each",
            Type::array(),
            Some(vec![Type::instance("Elem")]),
        );

        let recv_vtx = genv.new_vertex();
        let arr_src = genv.new_source(Type::array());
        genv.add_edge(arr_src, recv_vtx);

        let param_vtx = genv.new_vertex();

        let box_id = genv.alloc_box_id();
        let block_box = BlockParameterTypeBox::new(
            box_id,
            recv_vtx,
            "each".to_string(),
            vec![param_vtx],
        );
        genv.register_box(box_id, Box::new(block_box));

        genv.run_all();

        // Block parameter should remain untyped (type variable skipped)
        assert_eq!(genv.get_vertex(param_vtx).unwrap().show(), "untyped");
    }

    #[test]
    fn test_block_param_multiple_params() {
        let mut genv = GlobalEnv::new();

        // Register a method with multiple block params
        genv.register_builtin_method_with_block(
            Type::string(),
            "each_with_index",
            Type::string(),
            Some(vec![Type::string(), Type::integer()]),
        );

        let recv_vtx = genv.new_vertex();
        let str_src = genv.new_source(Type::string());
        genv.add_edge(str_src, recv_vtx);

        let param1_vtx = genv.new_vertex();
        let param2_vtx = genv.new_vertex();

        let box_id = genv.alloc_box_id();
        let block_box = BlockParameterTypeBox::new(
            box_id,
            recv_vtx,
            "each_with_index".to_string(),
            vec![param1_vtx, param2_vtx],
        );
        genv.register_box(box_id, Box::new(block_box));

        genv.run_all();

        // Both params should have their types
        assert_eq!(genv.get_vertex(param1_vtx).unwrap().show(), "String");
        assert_eq!(genv.get_vertex(param2_vtx).unwrap().show(), "Integer");
    }

    #[test]
    fn test_block_param_type_variable_resolved() {
        let mut genv = GlobalEnv::new();

        // Register Array#each with block param type Elem (type variable)
        genv.register_builtin_method_with_block(
            Type::array(),
            "each",
            Type::array(),
            Some(vec![Type::instance("Elem")]),
        );

        // Create receiver vertex with Array[Integer] type
        let recv_vtx = genv.new_vertex();
        let arr_src = genv.new_source(Type::array_of(Type::integer()));
        genv.add_edge(arr_src, recv_vtx);

        let param_vtx = genv.new_vertex();

        let box_id = genv.alloc_box_id();
        let block_box = BlockParameterTypeBox::new(
            box_id,
            recv_vtx,
            "each".to_string(),
            vec![param_vtx],
        );
        genv.register_box(box_id, Box::new(block_box));

        genv.run_all();

        // Block parameter should be Integer (resolved from Array[Integer])
        assert_eq!(genv.get_vertex(param_vtx).unwrap().show(), "Integer");
    }

    #[test]
    fn test_hash_type_variable_resolved() {
        let mut genv = GlobalEnv::new();

        // Register Hash#each with block param types K, V
        genv.register_builtin_method_with_block(
            Type::hash(),
            "each",
            Type::hash(),
            Some(vec![Type::instance("K"), Type::instance("V")]),
        );

        // Create receiver vertex with Hash[String, Integer] type
        let recv_vtx = genv.new_vertex();
        let hash_src = genv.new_source(Type::hash_of(Type::string(), Type::integer()));
        genv.add_edge(hash_src, recv_vtx);

        let key_vtx = genv.new_vertex();
        let value_vtx = genv.new_vertex();

        let box_id = genv.alloc_box_id();
        let block_box = BlockParameterTypeBox::new(
            box_id,
            recv_vtx,
            "each".to_string(),
            vec![key_vtx, value_vtx],
        );
        genv.register_box(box_id, Box::new(block_box));

        genv.run_all();

        // Block parameters should be resolved from Hash[String, Integer]
        assert_eq!(genv.get_vertex(key_vtx).unwrap().show(), "String");
        assert_eq!(genv.get_vertex(value_vtx).unwrap().show(), "Integer");
    }
}
