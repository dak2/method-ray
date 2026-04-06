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

/// Receiver type variables are resolved by position-matching against the receiver's Generic type_args.
fn is_type_variable_name(name: &str) -> bool {
    matches!(
        name,
        "Elem" | "K" | "V" | "T" | "Element" | "Key" | "Value"
    )
}

/// Block output type variables (e.g., U in `map { -> U }`) cannot be resolved from the receiver
/// and are substituted by BlockReturnTypeBox using the block body's return type.
fn is_block_type_variable_name(name: &str) -> bool {
    matches!(name, "U" | "A" | "B" | "Out" | "In")
}

/// Resolve type variables in a return type using the receiver's type args.
///
/// When `block_return_type` is None, block type variables cause None to be returned
/// (deferring to BlockReturnTypeBox). When provided, block type variables are
/// substituted with the given type.
fn resolve_return_type(
    return_type: &Type,
    recv_ty: &Type,
    block_return_type: Option<&Type>,
) -> Option<Type> {
    match return_type {
        Type::Instance { name } if is_block_type_variable_name(name.full_name()) => {
            block_return_type.cloned()
        }
        Type::Instance { name } if is_type_variable_name(name.full_name()) => {
            BlockParameterTypeBox::resolve_type_variable(return_type, recv_ty)
        }
        Type::Generic { name, type_args } => {
            let mut resolved_args = Vec::with_capacity(type_args.len());
            for arg in type_args {
                match arg {
                    Type::Instance { name: arg_name }
                        if is_block_type_variable_name(arg_name.full_name()) =>
                    {
                        match block_return_type {
                            Some(brt) => resolved_args.push(brt.clone()),
                            None => return None,
                        }
                    }
                    Type::Instance { name: arg_name }
                        if is_type_variable_name(arg_name.full_name()) =>
                    {
                        match BlockParameterTypeBox::resolve_type_variable(arg, recv_ty) {
                            Some(resolved) => resolved_args.push(resolved),
                            None => return None,
                        }
                    }
                    Type::Generic { .. } => {
                        match resolve_return_type(arg, recv_ty, block_return_type) {
                            Some(resolved) => resolved_args.push(resolved),
                            None => return None,
                        }
                    }
                    _ => resolved_args.push(arg.clone()),
                }
            }
            Some(Type::Generic {
                name: name.clone(),
                type_args: resolved_args,
            })
        }
        _ => Some(return_type.clone()),
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
                // Builtin/RBS method: resolve type variables from receiver's type args.
                // If unresolvable variables remain (e.g., block's U in map → Array[U]),
                // skip — BlockReturnTypeBox will add the resolved type.
                if let Some(resolved) = resolve_return_type(&method_info.return_type, recv_ty, None) {
                    let ret_src_id = genv.new_source(resolved);
                    changes.add_edge(ret_src_id, self.ret);
                }
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

    /// Try to resolve a type variable from receiver's type arguments.
    ///
    /// For `Array[Integer]#each { |x| }`, the block param type is `Elem`.
    /// This resolves `Elem` → `Integer` using Array's type argument.
    ///
    /// Type variable mapping for common generic classes:
    /// - Array[Elem]: Elem → type_args[0]
    /// - Hash[K, V]: K → type_args[0], V → type_args[1]
    pub(crate) fn resolve_type_variable(ty: &Type, recv_ty: &Type) -> Option<Type> {
        let type_var_name = match ty {
            Type::Instance { name } if is_type_variable_name(name.full_name()) => {
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
                                if is_type_variable_name(name.full_name())
                                    || is_block_type_variable_name(name.full_name())
                                {
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

/// Box for propagating block return type to method's generic return type.
///
/// For `[1,2].map { |x| x.to_s }`:
/// - Observes block body's return vertex type (String)
/// - Resolves method's RBS return type (Array[Elem])
/// - Substitutes: Elem → block_return → Array[String]
/// - Adds edge to method's return vertex
pub struct BlockReturnTypeBox {
    id: BoxId,
    recv_vtx: VertexId,
    method_name: String,
    block_return_vtx: VertexId,
    method_return_vtx: VertexId,
    reschedule_count: u8,
}

impl BlockReturnTypeBox {
    pub fn new(
        id: BoxId,
        recv_vtx: VertexId,
        method_name: String,
        block_return_vtx: VertexId,
        method_return_vtx: VertexId,
    ) -> Self {
        Self {
            id,
            recv_vtx,
            method_name,
            block_return_vtx,
            method_return_vtx,
            reschedule_count: 0,
        }
    }

    fn try_reschedule(&mut self, changes: &mut ChangeSet) {
        if self.reschedule_count < MAX_RESCHEDULE_COUNT {
            self.reschedule_count += 1;
            changes.reschedule(self.id);
        }
    }
}

impl BoxTrait for BlockReturnTypeBox {
    fn id(&self) -> BoxId {
        self.id
    }

    fn ret(&self) -> VertexId {
        self.method_return_vtx
    }

    fn run(&mut self, genv: &mut GlobalEnv, changes: &mut ChangeSet) {
        let Some(recv_types) = genv.get_receiver_types(self.recv_vtx) else {
            return;
        };
        if recv_types.is_empty() {
            self.try_reschedule(changes);
            return;
        }

        let block_return_types = match genv.get_receiver_types(self.block_return_vtx) {
            Some(types) if !types.is_empty() => types,
            _ => {
                self.try_reschedule(changes);
                return;
            }
        };

        let block_return_union = Type::union_of(block_return_types);

        for recv_ty in &recv_types {
            let Some(info) = genv.resolve_method(recv_ty, &self.method_name) else {
                continue;
            };

            // Reuse resolve_return_type with block_return_union to substitute
            // both receiver type variables and block output type variables in one pass.
            if let Some(resolved) =
                resolve_return_type(&info.return_type, recv_ty, Some(&block_return_union))
            {
                let src = genv.new_source(resolved);
                changes.add_edge(src, self.method_return_vtx);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::QualifiedName;

    #[test]
    fn test_is_type_variable_name() {
        assert!(is_type_variable_name("Elem"));
        assert!(is_type_variable_name("K"));
        assert!(is_type_variable_name("V"));
        assert!(is_type_variable_name("T"));
        assert!(!is_type_variable_name("U"));
        assert!(!is_type_variable_name("String"));
        assert!(!is_type_variable_name("In"));
    }

    #[test]
    fn test_is_block_type_variable_name() {
        assert!(is_block_type_variable_name("U"));
        assert!(is_block_type_variable_name("A"));
        assert!(is_block_type_variable_name("B"));
        assert!(is_block_type_variable_name("Out"));
        assert!(is_block_type_variable_name("In"));
        assert!(!is_block_type_variable_name("Elem"));
        assert!(!is_block_type_variable_name("String"));
    }

    #[test]
    fn test_resolve_return_type_passthrough() {
        let recv = Type::instance("String");
        let ret = Type::instance("Integer");
        assert_eq!(resolve_return_type(&ret, &recv, None), Some(Type::instance("Integer")));
    }

    #[test]
    fn test_resolve_return_type_receiver_type_variable() {
        // Array[Integer]#first → Elem → Integer
        let recv = Type::Generic {
            name: QualifiedName::simple("Array"),
            type_args: vec![Type::instance("Integer")],
        };
        let ret = Type::instance("Elem");
        assert_eq!(resolve_return_type(&ret, &recv, None), Some(Type::instance("Integer")));
    }

    #[test]
    fn test_resolve_return_type_block_type_variable_returns_none() {
        let recv = Type::instance("Array");
        let ret = Type::instance("U");
        assert_eq!(resolve_return_type(&ret, &recv, None), None);
    }

    #[test]
    fn test_resolve_return_type_generic_with_block_variable() {
        // Array[U] should return None (U is a block type variable)
        let recv = Type::Generic {
            name: QualifiedName::simple("Array"),
            type_args: vec![Type::instance("Integer")],
        };
        let ret = Type::Generic {
            name: QualifiedName::simple("Array"),
            type_args: vec![Type::instance("U")],
        };
        assert_eq!(resolve_return_type(&ret, &recv, None), None);
    }

    #[test]
    fn test_resolve_return_type_generic_with_resolvable_variable() {
        // Array[Elem] with recv Array[String] → Array[String]
        let recv = Type::Generic {
            name: QualifiedName::simple("Array"),
            type_args: vec![Type::instance("String")],
        };
        let ret = Type::Generic {
            name: QualifiedName::simple("Array"),
            type_args: vec![Type::instance("Elem")],
        };
        let expected = Type::Generic {
            name: QualifiedName::simple("Array"),
            type_args: vec![Type::instance("String")],
        };
        assert_eq!(resolve_return_type(&ret, &recv, None), Some(expected));
    }

    #[test]
    fn test_resolve_return_type_nested_generic_resolvable() {
        // Hash[K, Array[Elem]] with recv Hash[String, Integer]
        // K → String (Hash mapping), Elem → String (generic fallback: index 0)
        let recv = Type::Generic {
            name: QualifiedName::simple("Hash"),
            type_args: vec![Type::instance("String"), Type::instance("Integer")],
        };
        let ret = Type::Generic {
            name: QualifiedName::simple("Hash"),
            type_args: vec![
                Type::instance("K"),
                Type::Generic {
                    name: QualifiedName::simple("Array"),
                    type_args: vec![Type::instance("Elem")],
                },
            ],
        };
        let expected = Type::Generic {
            name: QualifiedName::simple("Hash"),
            type_args: vec![
                Type::instance("String"),
                Type::Generic {
                    name: QualifiedName::simple("Array"),
                    type_args: vec![Type::instance("String")],
                },
            ],
        };
        assert_eq!(resolve_return_type(&ret, &recv, None), Some(expected));
    }

    #[test]
    fn test_resolve_return_type_nested_generic_with_block_var() {
        // Array[Array[U]] with recv Array[Integer] → None (U is block variable)
        let recv = Type::Generic {
            name: QualifiedName::simple("Array"),
            type_args: vec![Type::instance("Integer")],
        };
        let ret = Type::Generic {
            name: QualifiedName::simple("Array"),
            type_args: vec![Type::Generic {
                name: QualifiedName::simple("Array"),
                type_args: vec![Type::instance("U")],
            }],
        };
        assert_eq!(resolve_return_type(&ret, &recv, None), None);
    }

    #[test]
    fn test_resolve_return_type_block_variable_with_substitution() {
        // U with block_return_type=String → String
        let recv = Type::instance("Array");
        let ret = Type::instance("U");
        let brt = Type::instance("String");
        assert_eq!(
            resolve_return_type(&ret, &recv, Some(&brt)),
            Some(Type::instance("String"))
        );
    }

    #[test]
    fn test_resolve_return_type_generic_block_variable_with_substitution() {
        // Array[U] with block_return_type=String → Array[String]
        let recv = Type::Generic {
            name: QualifiedName::simple("Array"),
            type_args: vec![Type::instance("Integer")],
        };
        let ret = Type::Generic {
            name: QualifiedName::simple("Array"),
            type_args: vec![Type::instance("U")],
        };
        let brt = Type::instance("String");
        let expected = Type::Generic {
            name: QualifiedName::simple("Array"),
            type_args: vec![Type::instance("String")],
        };
        assert_eq!(resolve_return_type(&ret, &recv, Some(&brt)), Some(expected));
    }

    #[test]
    fn test_resolve_return_type_nested_generic_all_resolvable() {
        // Hash[K, V] with recv Hash[String, Integer] → Hash[String, Integer]
        let recv = Type::Generic {
            name: QualifiedName::simple("Hash"),
            type_args: vec![Type::instance("String"), Type::instance("Integer")],
        };
        let ret = Type::Generic {
            name: QualifiedName::simple("Hash"),
            type_args: vec![Type::instance("K"), Type::instance("V")],
        };
        let expected = Type::Generic {
            name: QualifiedName::simple("Hash"),
            type_args: vec![Type::instance("String"), Type::instance("Integer")],
        };
        assert_eq!(resolve_return_type(&ret, &recv, None), Some(expected));
    }
}
