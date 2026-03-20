//! Operators - logical operator type inference (&&, ||, !)

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};
use crate::types::Type;
use ruby_prism::{AndNode, Node, OrNode};

use super::install::install_node;

/// Merge two branch nodes into a union type vertex.
fn process_binary_logical_op<'a>(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    left: Node<'a>,
    right: Node<'a>,
) -> Option<VertexId> {
    let result_vtx = genv.new_vertex();

    if let Some(vtx) = install_node(genv, lenv, changes, source, &left) {
        genv.add_edge(vtx, result_vtx);
    }

    if let Some(vtx) = install_node(genv, lenv, changes, source, &right) {
        genv.add_edge(vtx, result_vtx);
    }

    Some(result_vtx)
}

/// Process AndNode (a && b): Union(type(a), type(b))
///
/// Runtime: if `a` is falsy, returns `a`; otherwise returns `b`.
/// Static: conservatively produce Union(type(a), type(b)).
pub(crate) fn process_and_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    and_node: &AndNode,
) -> Option<VertexId> {
    process_binary_logical_op(genv, lenv, changes, source, and_node.left(), and_node.right())
}

/// Process OrNode (a || b): Union(type(a), type(b))
///
/// Runtime: if `a` is truthy, returns `a`; otherwise returns `b`.
/// Static: conservatively produce Union(type(a), type(b)).
pub(crate) fn process_or_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    or_node: &OrNode,
) -> Option<VertexId> {
    process_binary_logical_op(genv, lenv, changes, source, or_node.left(), or_node.right())
}

/// Process not operator (!expr): TrueClass | FalseClass
///
/// In ruby-prism, `!expr` is represented as a CallNode with method name "!".
/// Static approximation: we cannot determine the receiver's truthiness at
/// compile time, so conservatively return TrueClass | FalseClass for any `!` call.
/// In practice, `!nil` and `!false` are always true, but we do not track that here.
///
/// Receiver side effects are already analyzed by the caller (process_needs_child).
///
/// TODO: Ruby allows overriding `BasicObject#!`. Currently we always return
/// TrueClass | FalseClass, ignoring user-defined `!` methods. If needed, look up
/// the receiver's RBS definition and use its return type instead.
pub(crate) fn process_not_operator(genv: &mut GlobalEnv) -> VertexId {
    let result_vtx = genv.new_vertex();
    let true_vtx = genv.new_source(Type::instance("TrueClass"));
    let false_vtx = genv.new_source(Type::instance("FalseClass"));
    genv.add_edge(true_vtx, result_vtx);
    genv.add_edge(false_vtx, result_vtx);
    result_vtx
}
