//! Node Dispatch - Dispatch AST nodes to appropriate handlers
//!
//! This module handles the pattern matching of Ruby AST nodes
//! and dispatches them to specialized handlers.

use std::collections::HashMap;

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{BlockParameterTypeBox, ChangeSet, VertexId};
use crate::source_map::SourceLocation;
use crate::types::Type;
use ruby_prism::Node;

use super::bytes_to_name;
use super::calls::install_method_call;
use super::compound_assignments::{CompoundOp, CompoundVarKind};
use super::variables::{
    install_class_var_read, install_class_var_write, install_constant_read, install_constant_write,
    install_global_var_read, install_global_var_write, install_ivar_read, install_ivar_write,
    install_local_var_read, install_local_var_write, install_self,
};

/// Collect positional and keyword arguments from AST argument nodes.
///
/// Shared by method calls (`dispatch.rs`) and super calls (`super_calls.rs`).
pub(crate) fn collect_arguments<'a>(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    args: impl Iterator<Item = ruby_prism::Node<'a>>,
) -> (Vec<VertexId>, Option<HashMap<String, VertexId>>) {
    let mut positional: Vec<VertexId> = Vec::new();
    let mut keyword: HashMap<String, VertexId> = HashMap::new();

    for arg in args {
        if let Some(kw_hash) = arg.as_keyword_hash_node() {
            for element in kw_hash.elements().iter() {
                let assoc = match element.as_assoc_node() {
                    Some(a) => a,
                    None => continue,
                };
                let name = match assoc.key().as_symbol_node() {
                    Some(sym) => bytes_to_name(sym.unescaped()),
                    None => continue,
                };
                if let Some(vtx) =
                    super::install::install_node(genv, lenv, changes, source, &assoc.value())
                {
                    keyword.insert(name, vtx);
                }
            }
        } else if let Some(vtx) = super::install::install_node(genv, lenv, changes, source, &arg) {
            positional.push(vtx);
        }
    }

    let kw = (!keyword.is_empty()).then_some(keyword);
    (positional, kw)
}

/// Kind of attr_* declaration
#[derive(Debug, Clone, Copy)]
pub(crate) enum AttrKind {
    Reader,
    Writer,
    Accessor,
}

/// Result of dispatching a simple node (no child processing needed)
pub(crate) enum DispatchResult {
    /// Node produced a vertex
    Vertex(VertexId),
    /// Node was not handled
    NotHandled,
}

/// Kind of child processing needed
pub(crate) enum NeedsChildKind<'a> {
    /// Instance variable write: need to process value, then call install_ivar_write
    IvarWrite { ivar_name: String, value: Node<'a> },
    /// Class variable write: need to process value, then call install_class_var_write
    ClassVarWrite { cvar_name: String, value: Node<'a> },
    /// Local variable write: need to process value, then call install_local_var_write
    LocalVarWrite { var_name: String, value: Node<'a> },
    /// Global variable write: need to process value, then call install_global_var_write
    GlobalVarWrite { gvar_name: String, value: Node<'a> },
    ConstantWrite { const_name: String, value: Node<'a> },
    CompoundWrite {
        var_kind: CompoundVarKind,
        op: CompoundOp,
        value: Node<'a>,
    },
    /// Proc/lambda literals
    ProcLiteral { block: Node<'a> },
    /// Method call: need to process receiver, then call finish_method_call
    MethodCall {
        receiver: Node<'a>,
        method_name: String,
        location: SourceLocation,
        /// Optional block attached to the method call
        block: Option<Node<'a>>,
        /// Arguments to the method call
        arguments: Vec<Node<'a>>,
        /// Whether this is a safe navigation call (`&.`)
        safe_navigation: bool,
    },
    /// Implicit self method call: method call without explicit receiver (implicit self)
    ImplicitSelfCall {
        method_name: String,
        location: SourceLocation,
        block: Option<Node<'a>>,
        arguments: Vec<Node<'a>>,
    },
    /// attr_reader / attr_writer / attr_accessor declaration
    AttrDeclaration {
        kind: AttrKind,
        attr_names: Vec<String>,
    },
    /// include / extend declaration: `include Greetable`, `extend ClassMethods`
    ModuleMixinDeclaration {
        module_names: Vec<String>,
        kind: MixinKind,
    },
}

/// Kind of module mixin (include or extend)
#[derive(Debug, Clone, Copy)]
pub(crate) enum MixinKind {
    Include,
    Extend,
}

/// First pass: check if node can be handled immediately without child processing
///
/// Note: Literals (including Array) are handled in install.rs via install_literal
/// because Array literals need child processing for element type inference.
pub(crate) fn dispatch_simple(genv: &mut GlobalEnv, lenv: &mut LocalEnv, node: &Node) -> DispatchResult {
    // Instance variable read: @name
    if let Some(ivar_read) = node.as_instance_variable_read_node() {
        let ivar_name = bytes_to_name(ivar_read.name().as_slice());
        return match install_ivar_read(genv, &ivar_name) {
            Some(vtx) => DispatchResult::Vertex(vtx),
            None => DispatchResult::NotHandled,
        };
    }

    // Class variable read: @@name
    // Ruby: uninitialized class variables raise NameError at runtime.
    // We fall back to nil so downstream method calls are still type-checked
    // rather than silently skipped. This may produce nil-union types when
    // reads precede writes in source order.
    if let Some(cvar_read) = node.as_class_variable_read_node() {
        let cvar_name = bytes_to_name(cvar_read.name().as_slice());
        let vtx = install_class_var_read(genv, &cvar_name).unwrap_or_else(|| {
            let nil_vtx = genv.new_source(Type::Nil);
            install_class_var_write(genv, cvar_name, nil_vtx)
        });
        return DispatchResult::Vertex(vtx);
    }

    // Global variable read: $name
    // Ruby: uninitialized global variables are nil.
    // Register the nil vertex so repeated reads of the same uninitialized
    // variable reuse one vertex instead of allocating a new Source each time.
    if let Some(gvar_read) = node.as_global_variable_read_node() {
        let gvar_name = bytes_to_name(gvar_read.name().as_slice());
        let vtx = install_global_var_read(genv, &gvar_name).unwrap_or_else(|| {
            let nil_vtx = genv.new_source(Type::Nil);
            install_global_var_write(genv, gvar_name, nil_vtx)
        });
        return DispatchResult::Vertex(vtx);
    }

    // self
    if node.as_self_node().is_some() {
        return DispatchResult::Vertex(install_self(genv));
    }

    // Local variable read: x
    if let Some(read_node) = node.as_local_variable_read_node() {
        let var_name = bytes_to_name(read_node.name().as_slice());
        return match install_local_var_read(lenv, &var_name) {
            Some(vtx) => DispatchResult::Vertex(vtx),
            None => DispatchResult::NotHandled,
        };
    }

    // ConstantReadNode: User → Type::Singleton("User") or Type::Singleton("Api::User")
    if let Some(const_read) = node.as_constant_read_node() {
        let name = bytes_to_name(const_read.name().as_slice());

        if let Some(vtx) = install_constant_read(genv, &name) {
            return DispatchResult::Vertex(vtx);
        }

        let resolved_name = genv.scope_manager.lookup_constant(&name)
            .unwrap_or(name);
        let vtx = genv.new_source(Type::singleton(&resolved_name));
        return DispatchResult::Vertex(vtx);
    }

    // ConstantPathNode: Api::User → Type::Singleton("Api::User")
    if node.as_constant_path_node().is_some() {
        if let Some(name) = super::definitions::extract_constant_path(node) {
            let vtx = genv.new_source(Type::singleton(&name));
            return DispatchResult::Vertex(vtx);
        }
    }

    DispatchResult::NotHandled
}

/// Extract symbol names from attr_* arguments (e.g., `attr_reader :name, :email`)
fn extract_symbol_names(call_node: &ruby_prism::CallNode) -> Vec<String> {
    call_node
        .arguments()
        .map(|args| {
            args.arguments()
                .iter()
                .filter_map(|arg| {
                    arg.as_symbol_node().map(|sym| {
                        bytes_to_name(sym.unescaped())
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Extract module names from include/extend arguments
fn extract_mixin_module_names(call_node: &ruby_prism::CallNode) -> Vec<String> {
    call_node
        .arguments()
        .map(|args| {
            args.arguments()
                .iter()
                .filter_map(|arg| super::definitions::extract_constant_path(&arg))
                .collect()
        })
        .unwrap_or_default()
}

/// Check if node needs child processing
pub(crate) fn dispatch_needs_child<'a>(node: &Node<'a>, source: &str) -> Option<NeedsChildKind<'a>> {
    // Instance variable write: @name = value
    if let Some(ivar_write) = node.as_instance_variable_write_node() {
        let ivar_name = bytes_to_name(ivar_write.name().as_slice());
        return Some(NeedsChildKind::IvarWrite {
            ivar_name,
            value: ivar_write.value(),
        });
    }

    // Class variable write: @@name = value
    if let Some(cvar_write) = node.as_class_variable_write_node() {
        let cvar_name = bytes_to_name(cvar_write.name().as_slice());
        return Some(NeedsChildKind::ClassVarWrite {
            cvar_name,
            value: cvar_write.value(),
        });
    }

    // Global variable write: $name = value
    if let Some(gvar_write) = node.as_global_variable_write_node() {
        let gvar_name = bytes_to_name(gvar_write.name().as_slice());
        return Some(NeedsChildKind::GlobalVarWrite {
            gvar_name,
            value: gvar_write.value(),
        });
    }

    if let Some(const_write) = node.as_constant_write_node() {
        let const_name = bytes_to_name(const_write.name().as_slice());
        return Some(NeedsChildKind::ConstantWrite {
            const_name,
            value: const_write.value(),
        });
    }

    // Local variable write: x = value
    if let Some(write_node) = node.as_local_variable_write_node() {
        let var_name = bytes_to_name(write_node.name().as_slice());
        return Some(NeedsChildKind::LocalVarWrite {
            var_name,
            value: write_node.value(),
        });
    }

    macro_rules! dispatch_compound {
        ($node:expr, $as_method:ident, $var_kind:ident, operator) => {
            if let Some(n) = $node.$as_method() {
                return Some(NeedsChildKind::CompoundWrite {
                    var_kind: CompoundVarKind::$var_kind(bytes_to_name(n.name().as_slice())),
                    op: CompoundOp::Operator(bytes_to_name(n.binary_operator().as_slice())),
                    value: n.value(),
                });
            }
        };
        ($node:expr, $as_method:ident, $var_kind:ident, $op:ident) => {
            if let Some(n) = $node.$as_method() {
                return Some(NeedsChildKind::CompoundWrite {
                    var_kind: CompoundVarKind::$var_kind(bytes_to_name(n.name().as_slice())),
                    op: CompoundOp::$op,
                    value: n.value(),
                });
            }
        };
    }

    macro_rules! dispatch_compound_all {
        ($node:expr, $op_method:ident, $or_method:ident, $and_method:ident, $var_kind:ident) => {
            dispatch_compound!($node, $op_method, $var_kind, operator);
            dispatch_compound!($node, $or_method, $var_kind, Logical);
            dispatch_compound!($node, $and_method, $var_kind, Logical);
        };
    }

    dispatch_compound_all!(node,
        as_local_variable_operator_write_node, as_local_variable_or_write_node, as_local_variable_and_write_node, Local);
    dispatch_compound_all!(node,
        as_instance_variable_operator_write_node, as_instance_variable_or_write_node, as_instance_variable_and_write_node, Ivar);
    dispatch_compound_all!(node,
        as_class_variable_operator_write_node, as_class_variable_or_write_node, as_class_variable_and_write_node, ClassVar);
    dispatch_compound_all!(node,
        as_global_variable_operator_write_node, as_global_variable_or_write_node, as_global_variable_and_write_node, GlobalVar);
    dispatch_compound_all!(node,
        as_constant_operator_write_node, as_constant_or_write_node, as_constant_and_write_node, Constant);

    // Method call: x.upcase, x.each { |i| ... }, or name (implicit self)
    if let Some(call_node) = node.as_call_node() {
        let method_name = bytes_to_name(call_node.name().as_slice());
        let block = call_node.block();
        let arguments: Vec<Node<'a>> = call_node
            .arguments()
            .map(|args| args.arguments().iter().collect())
            .unwrap_or_default();

        if let Some(receiver) = call_node.receiver() {
            if method_name == "new" {
                if let Some(const_read) = receiver.as_constant_read_node() {
                    if const_read.name().as_slice() == b"Proc" {
                        if let Some(block_node) = block {
                            return Some(NeedsChildKind::ProcLiteral { block: block_node });
                        }
                    }
                }
            }

            // Explicit receiver: x.upcase, x.each { |i| ... }
            let prism_location = call_node
                .call_operator_loc()
                .unwrap_or_else(|| node.location());
            let location =
                SourceLocation::from_prism_location_with_source(&prism_location, source);

            return Some(NeedsChildKind::MethodCall {
                receiver,
                method_name,
                location,
                block,
                arguments,
                safe_navigation: call_node.is_safe_navigation(),
            });
        } else {
            // No receiver: implicit self method call (e.g., `name`, `puts "hello"`)

            if matches!(method_name.as_str(), "lambda" | "proc") {
                if let Some(block_node) = block {
                    return Some(NeedsChildKind::ProcLiteral { block: block_node });
                }
            }

            if let Some(kind) = match method_name.as_str() {
                "attr_reader" => Some(AttrKind::Reader),
                "attr_writer" => Some(AttrKind::Writer),
                "attr_accessor" => Some(AttrKind::Accessor),
                _ => None,
            } {
                let attr_names = extract_symbol_names(&call_node);
                if !attr_names.is_empty() {
                    return Some(NeedsChildKind::AttrDeclaration { kind, attr_names });
                }
                return None;
            }

            let mixin_kind = match method_name.as_str() {
                "include" => Some(MixinKind::Include),
                "extend" => Some(MixinKind::Extend),
                _ => None,
            };

            if let Some(kind) = mixin_kind {
                let module_names = extract_mixin_module_names(&call_node);
                if !module_names.is_empty() {
                    return Some(NeedsChildKind::ModuleMixinDeclaration { module_names, kind });
                }
                return None;
            }

            let prism_location = call_node
                .message_loc()
                .unwrap_or_else(|| node.location());
            let location =
                SourceLocation::from_prism_location_with_source(&prism_location, source);

            return Some(NeedsChildKind::ImplicitSelfCall {
                method_name,
                location,
                block,
                arguments,
            });
        }
    }

    None
}

/// Process a node that needs child processing
///
/// This function handles the second phase of two-phase dispatch:
/// 1. `dispatch_needs_child` identifies the node kind and extracts data
/// 2. `process_needs_child` processes child nodes and completes the operation
pub(crate) fn process_needs_child(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    kind: NeedsChildKind,
) -> Option<VertexId> {
    match kind {
        NeedsChildKind::IvarWrite { ivar_name, value } => {
            let value_vtx = super::install::install_node(genv, lenv, changes, source, &value)?;
            Some(install_ivar_write(genv, ivar_name, value_vtx))
        }
        NeedsChildKind::ClassVarWrite { cvar_name, value } => {
            let value_vtx = super::install::install_node(genv, lenv, changes, source, &value)?;
            Some(install_class_var_write(genv, cvar_name, value_vtx))
        }
        NeedsChildKind::GlobalVarWrite { gvar_name, value } => {
            let value_vtx = super::install::install_node(genv, lenv, changes, source, &value)?;
            Some(install_global_var_write(genv, gvar_name, value_vtx))
        }
        NeedsChildKind::ConstantWrite { const_name, value } => {
            let value_vtx = super::install::install_node(genv, lenv, changes, source, &value)?;
            Some(install_constant_write(genv, const_name, value_vtx))
        }
        NeedsChildKind::LocalVarWrite { var_name, value } => {
            let value_vtx = super::install::install_node(genv, lenv, changes, source, &value)?;
            Some(install_local_var_write(genv, lenv, changes, var_name, value_vtx))
        }
        NeedsChildKind::CompoundWrite { var_kind, op, value } => {
            let value_vtx = super::install::install_node(genv, lenv, changes, source, &value)?;
            Some(super::compound_assignments::process_compound_write(
                genv, lenv, changes, var_kind, op, value_vtx,
            ))
        }
        NeedsChildKind::ProcLiteral { block } => {
            if let Some(block_node) = block.as_block_node() {
                super::lambdas::process_block_as_proc(genv, lenv, changes, source, &block_node)
            } else {
                None
            }
        }
        NeedsChildKind::MethodCall {
            receiver,
            method_name,
            location,
            block,
            arguments,
            safe_navigation,
        } => {
            let recv_vtx = super::install::install_node(genv, lenv, changes, source, &receiver)?;
            process_method_call_common(
                genv, lenv, changes, source,
                MethodCallContext { recv_vtx, method_name, location, block, arguments, safe_navigation },
            )
        }
        NeedsChildKind::ImplicitSelfCall {
            method_name,
            location,
            block,
            arguments,
        } => {
            // Use qualified name to match method registration in definitions.rs
            let recv_vtx = if let Some(name) = genv.scope_manager.current_qualified_name() {
                genv.new_source(Type::instance(&name))
            } else {
                genv.new_source(Type::instance("Object"))
            };
            process_method_call_common(
                genv, lenv, changes, source,
                // Implicit self calls cannot use safe navigation (`&.` requires explicit receiver)
                MethodCallContext { recv_vtx, method_name, location, block, arguments, safe_navigation: false },
            )
        }
        NeedsChildKind::AttrDeclaration { kind, attr_names } => {
            super::attributes::process_attr_declaration(genv, kind, attr_names);
            None
        }
        NeedsChildKind::ModuleMixinDeclaration { module_names, kind } => {
            if let Some(class_name) = genv.scope_manager.current_qualified_name() {
                // Ruby processes `include/extend A, B` right-to-left (B first, then A on top),
                // so A ends up with higher MRO priority. Reverse to match this behavior.
                for module_name in module_names.iter().rev() {
                    match kind {
                        MixinKind::Include => genv.record_include(&class_name, module_name),
                        MixinKind::Extend => genv.record_extend(&class_name, module_name),
                    }
                }
            }
            None
        }
    }
}

/// Bundled parameters for method call processing
struct MethodCallContext<'a> {
    recv_vtx: VertexId,
    method_name: String,
    location: SourceLocation,
    block: Option<Node<'a>>,
    arguments: Vec<Node<'a>>,
    safe_navigation: bool,
}

/// MethodCall / ImplicitSelfCall common processing:
/// Handles argument processing, block processing, and MethodCallBox creation after recv_vtx is obtained
fn process_method_call_common<'a>(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    ctx: MethodCallContext<'a>,
) -> Option<VertexId> {
    let MethodCallContext {
        recv_vtx,
        method_name,
        location,
        block,
        arguments,
        safe_navigation,
    } = ctx;
    if method_name == "!" {
        return Some(super::operators::process_not_operator(genv));
    }

    let (positional_arg_vtxs, kwarg_vtxs) =
        collect_arguments(genv, lenv, changes, source, arguments.into_iter());

    if let Some(block_node) = block {
        if let Some(block) = block_node.as_block_node() {
            let param_vtxs = super::blocks::process_block_node_with_params(
                genv, lenv, changes, source, &block,
            );

            if !param_vtxs.is_empty() {
                let box_id = genv.alloc_box_id();
                let block_box = BlockParameterTypeBox::new(
                    box_id,
                    recv_vtx,
                    method_name.clone(),
                    param_vtxs,
                );
                genv.register_box(box_id, Box::new(block_box));
            }
        }
    }

    Some(finish_method_call(
        genv,
        recv_vtx,
        method_name,
        positional_arg_vtxs,
        kwarg_vtxs,
        location,
        safe_navigation,
    ))
}

/// Finish method call after receiver is processed
fn finish_method_call(
    genv: &mut GlobalEnv,
    recv_vtx: VertexId,
    method_name: String,
    arg_vtxs: Vec<VertexId>,
    kwarg_vtxs: Option<HashMap<String, VertexId>>,
    location: SourceLocation,
    safe_navigation: bool,
) -> VertexId {
    install_method_call(genv, recv_vtx, method_name, arg_vtxs, kwarg_vtxs, Some(location), safe_navigation)
}
