use std::collections::HashMap;

use crate::env::GlobalEnv;
use crate::graph::change_set::ChangeSet;
use crate::graph::vertex::VertexId;
use crate::source_map::SourceLocation;
use crate::types::Type;

/// Unique ID for Box
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BoxId(pub usize);

/// Box trait: represents constraints such as method calls
pub trait BoxTrait: Send + Sync {
    fn id(&self) -> BoxId;
    fn run(&mut self, genv: &mut GlobalEnv, changes: &mut ChangeSet);
    fn ret(&self) -> VertexId;
}

/// Propagate argument types to parameter vertices by adding edges
/// from each argument vertex to the corresponding parameter vertex.
fn propagate_arguments(
    arg_vtxs: &[VertexId],
    param_vtxs: Option<&[VertexId]>,
    changes: &mut ChangeSet,
) {
    for (arg_vtx, param_vtx) in arg_vtxs.iter().zip(param_vtxs.unwrap_or_default()) {
        changes.add_edge(*arg_vtx, *param_vtx);
    }
}

/// Propagate keyword argument types to keyword parameter vertices by name
fn propagate_keyword_arguments(
    kwarg_vtxs: Option<&HashMap<String, VertexId>>,
    kw_param_vtxs: Option<&HashMap<String, VertexId>>,
    changes: &mut ChangeSet,
) {
    let (Some(args), Some(params)) = (kwarg_vtxs, kw_param_vtxs) else {
        return;
    };
    for (name, arg_vtx) in args {
        if let Some(&param_vtx) = params.get(name) {
            changes.add_edge(*arg_vtx, param_vtx);
        }
    }
}

/// Box representing a method call
pub struct MethodCallBox {
    id: BoxId,
    recv: VertexId,
    method_name: String,
    ret: VertexId,
    arg_vtxs: Vec<VertexId>,
    kwarg_vtxs: Option<HashMap<String, VertexId>>,
    location: Option<SourceLocation>, // Source code location
    /// Whether this is a safe navigation call (`&.`)
    safe_navigation: bool,
    /// Number of times this box has been rescheduled
    reschedule_count: u8,
}

/// Maximum number of reschedules before giving up
const MAX_RESCHEDULE_COUNT: u8 = 3;

impl MethodCallBox {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: BoxId,
        recv: VertexId,
        method_name: String,
        ret: VertexId,
        arg_vtxs: Vec<VertexId>,
        kwarg_vtxs: Option<HashMap<String, VertexId>>,
        location: Option<SourceLocation>,
        safe_navigation: bool,
    ) -> Self {
        Self {
            id,
            recv,
            method_name,
            ret,
            arg_vtxs,
            kwarg_vtxs,
            location,
            safe_navigation,
            reschedule_count: 0,
        }
    }

    /// Reschedule this box for re-execution if the limit hasn't been reached.
    /// Handles cases where the receiver has no types yet (e.g., block parameters
    /// that get typed by a later box). If max reschedules are reached, the box
    /// is silently dropped (receiver type remains unknown).
    fn try_reschedule(&mut self, changes: &mut ChangeSet) {
        if self.reschedule_count < MAX_RESCHEDULE_COUNT {
            self.reschedule_count += 1;
            changes.reschedule(self.id);
        }
    }

    fn process_recv_type(
        &self,
        recv_ty: &Type,
        genv: &mut GlobalEnv,
        changes: &mut ChangeSet,
    ) {
        // Safe navigation (`&.`): skip nil receiver entirely.
        // Ruby's &. short-circuits: no method resolution, no argument evaluation, no error.
        // The nil return type is added in run() after processing all receiver types.
        if self.safe_navigation && matches!(recv_ty, Type::Nil) {
            return;
        }

        if let Type::Proc { return_vertex, param_vertices, .. } = recv_ty {
            if self.method_name == "call" {
                if let Some(merge_vtx) = return_vertex {
                    changes.add_edge(*merge_vtx, self.ret);
                }
                propagate_arguments(&self.arg_vtxs, Some(param_vertices), changes);
            }
            // TODO: Proc#arity, Proc#curry etc. not yet resolved via RBS
            return;
        }

        if let Some(method_info) = genv.resolve_method(recv_ty, &self.method_name) {
            if let Some(return_vtx) = method_info.return_vertex {
                // User-defined method: connect body's return vertex to call site
                changes.add_edge(return_vtx, self.ret);
                propagate_arguments(
                    &self.arg_vtxs,
                    method_info.param_vertices.as_deref(),
                    changes,
                );
                propagate_keyword_arguments(
                    self.kwarg_vtxs.as_ref(),
                    method_info.keyword_param_vertices.as_ref(),
                    changes,
                );
            } else {
                // Builtin/RBS method: create source with fixed return type
                let ret_src_id = genv.new_source(method_info.return_type.clone());
                changes.add_edge(ret_src_id, self.ret);
            }
        } else if self.method_name == "new" {
            self.handle_new_call(recv_ty, genv, changes);
        } else if !matches!(recv_ty, Type::Singleton { .. }) {
            // Singleton types with unresolved methods are silently skipped;
            // these are typically RBS class methods not yet supported.
            self.report_type_error(recv_ty, genv);
        }
    }

    /// Handle `.new` calls: singleton(Foo)#new produces instance(Foo),
    /// and propagates arguments to the `initialize` method's parameters.
    fn handle_new_call(
        &self,
        recv_ty: &Type,
        genv: &mut GlobalEnv,
        changes: &mut ChangeSet,
    ) {
        if let Type::Singleton { name } = recv_ty {
            let instance_type = Type::instance(name.full_name());

            let ret_src = genv.new_source(instance_type.clone());
            changes.add_edge(ret_src, self.ret);

            let init_info = genv.resolve_method(&instance_type, "initialize");
            propagate_arguments(
                &self.arg_vtxs,
                init_info.and_then(|info| info.param_vertices.as_deref()),
                changes,
            );
            propagate_keyword_arguments(
                self.kwarg_vtxs.as_ref(),
                init_info.and_then(|info| info.keyword_param_vertices.as_ref()),
                changes,
            );
        } else {
            self.report_type_error(recv_ty, genv);
        }
    }

    fn report_type_error(&self, recv_ty: &Type, genv: &mut GlobalEnv) {
        genv.record_type_error(
            recv_ty.clone(),
            self.method_name.clone(),
            self.location.clone(),
        );
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
        let Some(recv_types) = genv.get_receiver_types(self.recv) else {
            return;
        };

        if recv_types.is_empty() {
            self.try_reschedule(changes);
            return;
        }

        for recv_ty in &recv_types {
            self.process_recv_type(recv_ty, genv, changes);
        }

        // Safe navigation (`&.`): if receiver can be nil, return type includes nil
        if self.safe_navigation && recv_types.iter().any(|t| matches!(t, Type::Nil)) {
            let nil_src = genv.new_source(Type::Nil);
            changes.add_edge(nil_src, self.ret);
        }
    }
}

/// Box for resolving block parameter types from method call receiver
///
/// When a method with a block is called (e.g., `str.each_char { |c| ... }`),
/// this box resolves the block parameter types from the method's RBS definition
/// and propagates them to the block parameter vertices.
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
        let Some(recv_types) = genv.get_receiver_types(self.recv_vtx) else {
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
