//! Conditionals - if/unless/case type inference
//!
//! Collects types from each branch and merges them into a Union
//! via edges into a single result Vertex.

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};
use crate::types::Type;
use ruby_prism::{CaseNode, ElseNode, IfNode, Node, UnlessNode, WhenNode};

use super::install::{install_node, install_statements};

/// Process IfNode: if/elsif/else chain
pub(crate) fn process_if_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    if_node: &IfNode,
) -> Option<VertexId> {
    // Process predicate for side effects
    install_node(genv, lenv, changes, source, &if_node.predicate());

    let result_vtx = genv.new_vertex();

    // then branch
    let vtx_then = if_node
        .statements()
        .and_then(|stmts| install_statements(genv, lenv, changes, source, &stmts));
    if let Some(vtx) = vtx_then {
        genv.add_edge(vtx, result_vtx);
    }

    // elsif/else branch (subsequent)
    let has_else = if let Some(subsequent) = if_node.subsequent() {
        let vtx_sub = process_subsequent(genv, lenv, changes, source, &subsequent);
        if let Some(vtx) = vtx_sub {
            genv.add_edge(vtx, result_vtx);
        }
        true
    } else {
        false
    };

    // No else clause → add nil
    if !has_else {
        let nil_vtx = genv.new_source(Type::Nil);
        genv.add_edge(nil_vtx, result_vtx);
    }

    Some(result_vtx)
}

/// Process UnlessNode: unless/else
pub(crate) fn process_unless_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    unless_node: &UnlessNode,
) -> Option<VertexId> {
    // Process predicate for side effects
    install_node(genv, lenv, changes, source, &unless_node.predicate());

    let result_vtx = genv.new_vertex();

    // body branch
    let vtx_body = unless_node
        .statements()
        .and_then(|stmts| install_statements(genv, lenv, changes, source, &stmts));
    if let Some(vtx) = vtx_body {
        genv.add_edge(vtx, result_vtx);
    }

    // else clause
    let has_else = if let Some(else_node) = unless_node.else_clause() {
        let vtx_else = process_else_clause(genv, lenv, changes, source, &else_node);
        if let Some(vtx) = vtx_else {
            genv.add_edge(vtx, result_vtx);
        }
        true
    } else {
        false
    };

    // No else clause → add nil
    if !has_else {
        let nil_vtx = genv.new_source(Type::Nil);
        genv.add_edge(nil_vtx, result_vtx);
    }

    Some(result_vtx)
}

/// Process CaseNode: case/when/else
pub(crate) fn process_case_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    case_node: &CaseNode,
) -> Option<VertexId> {
    // Process predicate for side effects
    if let Some(pred) = case_node.predicate() {
        install_node(genv, lenv, changes, source, &pred);
    }

    let result_vtx = genv.new_vertex();

    // Process each when clause
    for condition in &case_node.conditions() {
        if let Some(when_node) = condition.as_when_node() {
            let vtx_when = process_when_clause(genv, lenv, changes, source, &when_node);
            if let Some(vtx) = vtx_when {
                genv.add_edge(vtx, result_vtx);
            }
        }
    }

    // else clause
    let has_else = if let Some(else_node) = case_node.else_clause() {
        let vtx_else = process_else_clause(genv, lenv, changes, source, &else_node);
        if let Some(vtx) = vtx_else {
            genv.add_edge(vtx, result_vtx);
        }
        true
    } else {
        false
    };

    // No else clause → add nil
    if !has_else {
        let nil_vtx = genv.new_source(Type::Nil);
        genv.add_edge(nil_vtx, result_vtx);
    }

    Some(result_vtx)
}

/// Process subsequent node (elsif chain or else)
fn process_subsequent(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    node: &Node,
) -> Option<VertexId> {
    // elsif: subsequent is another IfNode
    if let Some(if_node) = node.as_if_node() {
        return process_if_node(genv, lenv, changes, source, &if_node);
    }

    // else: subsequent is an ElseNode
    if let Some(else_node) = node.as_else_node() {
        return process_else_clause(genv, lenv, changes, source, &else_node);
    }

    None
}

/// Process ElseNode
fn process_else_clause(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    else_node: &ElseNode,
) -> Option<VertexId> {
    else_node
        .statements()
        .and_then(|stmts| install_statements(genv, lenv, changes, source, &stmts))
}

/// Process WhenNode
fn process_when_clause(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    when_node: &WhenNode,
) -> Option<VertexId> {
    // Process when conditions for side effects
    for cond in &when_node.conditions() {
        install_node(genv, lenv, changes, source, &cond);
    }

    when_node
        .statements()
        .and_then(|stmts| install_statements(genv, lenv, changes, source, &stmts))
}
