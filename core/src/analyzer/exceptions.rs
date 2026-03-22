//! Exceptions - begin/rescue/ensure type inference
//!
//! Collects types from each branch and merges them into a Union
//! via edges into a single result Vertex.
//! Applies the same MergeVertex pattern as conditionals.rs.

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};
use crate::types::Type;
use ruby_prism::{BeginNode, RescueModifierNode, RescueNode};

use super::bytes_to_name;
use super::install::{install_node, install_statements};

/// Process BeginNode: begin/rescue/else/ensure
///
/// Type aggregation rules:
///   - No rescue clause: return begin body type directly
///   - With else clause: else type + all rescue types → Union (begin body excluded)
///   - Without else clause: begin body type + all rescue types → Union
///   - Ensure clause: processed for side effects only, does not affect return type
pub(crate) fn process_begin_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    begin_node: &BeginNode,
) -> Option<VertexId> {
    let begin_vtx = begin_node
        .statements()
        .and_then(|s| install_statements(genv, lenv, changes, source, &s));

    let result = if let Some(rescue_node) = begin_node.rescue_clause() {
        let result_vtx = genv.new_vertex();

        process_rescue_chain(genv, lenv, changes, source, &rescue_node, result_vtx);

        if let Some(else_node) = begin_node.else_clause() {
            // With else: else type replaces begin body type (Ruby spec)
            let else_vtx = else_node
                .statements()
                .and_then(|s| install_statements(genv, lenv, changes, source, &s));
            if let Some(vtx) = else_vtx {
                genv.add_edge(vtx, result_vtx);
            }
        } else if let Some(vtx) = begin_vtx {
            genv.add_edge(vtx, result_vtx);
        }

        Some(result_vtx)
    } else {
        begin_vtx
    };

    // Ensure: side effects only, does not affect return type
    if let Some(ensure_node) = begin_node.ensure_clause() {
        if let Some(stmts) = ensure_node.statements() {
            let _ = install_statements(genv, lenv, changes, source, &stmts);
        }
    }

    result
}

/// Process RescueNode chain recursively.
/// Empty rescue body evaluates to nil.
fn process_rescue_chain(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    rescue_node: &RescueNode,
    result_vtx: VertexId,
) {
    let body_vtx = process_rescue_body(genv, lenv, changes, source, rescue_node);
    let vtx = body_vtx.unwrap_or_else(|| genv.new_source(Type::Nil));
    genv.add_edge(vtx, result_vtx);

    if let Some(next) = rescue_node.subsequent() {
        process_rescue_chain(genv, lenv, changes, source, &next, result_vtx);
    }
}

/// Extract the exception type from rescue node's exception class list.
/// Falls back to StandardError when no exceptions are specified or none can be resolved.
// TODO: Non-constant exception expressions (method calls, splats, variables) are silently skipped.
fn extract_exception_type(rescue_node: &RescueNode) -> Type {
    let types: Vec<Type> = rescue_node
        .exceptions()
        .iter()
        .filter_map(|exc| super::definitions::extract_constant_path(&exc))
        .map(|name| Type::instance(&name))
        .collect();

    if types.is_empty() {
        Type::instance("StandardError")
    } else {
        Type::union_of(types)
    }
}

/// Process a single RescueNode body.
/// Registers the rescue variable (=> e), processes the body,
/// then removes the variable from scope.
fn process_rescue_body(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    rescue_node: &RescueNode,
) -> Option<VertexId> {
    for exc in &rescue_node.exceptions() {
        install_node(genv, lenv, changes, source, &exc);
    }

    // Save/restore rescue variable binding (=> e)
    // TODO: Only LocalVariableTargetNode is handled; instance/global/class vars are not yet supported.
    let var_binding = if let Some(ref_node) = rescue_node.reference() {
        ref_node.as_local_variable_target_node().map(|target| {
            let name = bytes_to_name(target.name().as_slice());
            let saved = lenv.get_var(&name);
            let exception_vtx = genv.new_vertex();
            let exception_type = extract_exception_type(rescue_node);
            let exception_src = genv.new_source(exception_type);
            genv.add_edge(exception_src, exception_vtx);
            lenv.new_var(name.clone(), exception_vtx);
            (name, saved)
        })
    } else {
        None
    };

    let body_vtx = rescue_node
        .statements()
        .and_then(|s| install_statements(genv, lenv, changes, source, &s));

    if let Some((name, saved)) = var_binding {
        match saved {
            Some(prev_vtx) => lenv.new_var(name, prev_vtx),
            None => lenv.remove_var(&name),
        }
    }

    body_vtx
}

/// Process RescueModifierNode: `expression rescue rescue_expression`
pub(crate) fn process_rescue_modifier_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    node: &RescueModifierNode,
) -> Option<VertexId> {
    let result_vtx = genv.new_vertex();

    let expr_vtx = install_node(genv, lenv, changes, source, &node.expression());
    if let Some(vtx) = expr_vtx {
        genv.add_edge(vtx, result_vtx);
    }

    let rescue_vtx = install_node(genv, lenv, changes, source, &node.rescue_expression());
    if let Some(vtx) = rescue_vtx {
        genv.add_edge(vtx, result_vtx);
    }

    Some(result_vtx)
}
