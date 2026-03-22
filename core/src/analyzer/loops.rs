//! Loops - while/until loop type inference
//!
//! Ruby loop expressions evaluate to nil (except when break passes a value).
//! Break value propagation is not yet supported.

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};
use crate::types::Type;
use ruby_prism::{ForNode, UntilNode, WhileNode};

use super::bytes_to_name;
use super::install::{install_node, install_statements};

/// Process WhileNode: `while predicate; statements; end`
///
/// Returns nil. Traverses predicate and body to register method calls
/// and variable assignments in the type graph.
pub(crate) fn process_while_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    while_node: &WhileNode,
) -> Option<VertexId> {
    install_node(genv, lenv, changes, source, &while_node.predicate());

    if let Some(stmts) = while_node.statements() {
        install_statements(genv, lenv, changes, source, &stmts);
    }

    Some(genv.new_source(Type::Nil))
}

/// Process UntilNode: `until predicate; statements; end`
///
/// Returns nil. Traverses predicate and body to register method calls
/// and variable assignments in the type graph.
pub(crate) fn process_until_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    until_node: &UntilNode,
) -> Option<VertexId> {
    install_node(genv, lenv, changes, source, &until_node.predicate());

    if let Some(stmts) = until_node.statements() {
        install_statements(genv, lenv, changes, source, &stmts);
    }

    Some(genv.new_source(Type::Nil))
}

/// Process ForNode: `for index in collection; statements; end`
///
/// Ruby's `for` does NOT create a new scope — the loop variable persists
/// after the loop. This differs from `collection.each { |x| }` which
/// creates a block scope.
///
/// Returns nil (consistent with while/until; Ruby's for returns the
/// collection, but the return value is rarely used in practice).
pub(crate) fn process_for_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    for_node: &ForNode,
) -> Option<VertexId> {
    let collection_vtx = install_node(genv, lenv, changes, source, &for_node.collection());

    // TODO: MultiTargetNode (e.g., `for a, b in [[1, "x"]]`) is not yet supported
    if let Some(target) = for_node.index().as_local_variable_target_node() {
        let name = bytes_to_name(target.name().as_slice());
        let var_vtx = genv.new_vertex();

        // Array[T] or Range[T] → loop var gets T
        let elem_type = collection_vtx
            .and_then(|vtx| genv.get_source(vtx))
            .and_then(|src| src.ty.type_args())
            .and_then(|args| args.first().cloned());
        if let Some(ty) = elem_type {
            let elem_src = genv.new_source(ty);
            genv.add_edge(elem_src, var_vtx);
        }

        lenv.new_var(name, var_vtx);
    }

    if let Some(stmts) = for_node.statements() {
        install_statements(genv, lenv, changes, source, &stmts);
    }

    Some(genv.new_source(Type::Nil))
}
