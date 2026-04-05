 //! Conditionals - if/unless/case type inference
//!
//! Collects types from each branch and merges them into a Union
//! via edges into a single result Vertex.

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};
use crate::types::Type;
use ruby_prism::{
    ArrayPatternNode, CapturePatternNode, CaseMatchNode, CaseNode, ElseNode, FindPatternNode,
    HashPatternNode, IfNode, InNode, Node, UnlessNode, WhenNode,
};

use super::bytes_to_name;
use super::install::{install_node, install_statements};
use super::variables::install_local_var_write;

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

/// Process CaseMatchNode: case/in pattern matching
pub(crate) fn process_case_match_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    node: &CaseMatchNode,
) -> Option<VertexId> {
    let predicate_vtx = node
        .predicate()
        .and_then(|pred| install_node(genv, lenv, changes, source, &pred));

    let result_vtx = genv.new_vertex();

    for condition in &node.conditions() {
        if let Some(in_node) = condition.as_in_node() {
            let vtx = process_in_clause(genv, lenv, changes, source, &in_node, predicate_vtx);
            if let Some(vtx) = vtx {
                genv.add_edge(vtx, result_vtx);
            }
        }
    }

    let has_else = if let Some(else_node) = node.else_clause() {
        let vtx = process_else_clause(genv, lenv, changes, source, &else_node);
        if let Some(vtx) = vtx {
            genv.add_edge(vtx, result_vtx);
        }
        true
    } else {
        false
    };

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

/// Process InNode clause body
fn process_in_clause(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    in_node: &InNode,
    predicate_vtx: Option<VertexId>,
) -> Option<VertexId> {
    process_pattern(genv, lenv, changes, source, &in_node.pattern(), predicate_vtx);
    in_node
        .statements()
        .and_then(|s| install_statements(genv, lenv, changes, source, &s))
}

/// Dispatch pattern processing based on pattern type
fn process_pattern(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    pattern: &Node,
    predicate_vtx: Option<VertexId>,
) {
    // Guard pattern (in x if condition)
    if let Some(if_node) = pattern.as_if_node() {
        if let Some(stmts) = if_node.statements() {
            for stmt in &stmts.body() {
                process_pattern(genv, lenv, changes, source, &stmt, predicate_vtx);
            }
        }
        install_node(genv, lenv, changes, source, &if_node.predicate());
        return;
    }

    if let Some(cap) = pattern.as_capture_pattern_node() {
        process_capture_pattern(genv, lenv, changes, source, &cap);
        return;
    }

    // ImplicitNode: hash shorthand pattern { name: } wraps LocalVariableTargetNode
    if let Some(implicit) = pattern.as_implicit_node() {
        process_pattern(genv, lenv, changes, source, &implicit.value(), predicate_vtx);
        return;
    }

    // LocalVariableTargetNode: single variable binding (in x)
    if let Some(target) = pattern.as_local_variable_target_node() {
        let var_name = bytes_to_name(target.name().as_slice());
        let type_vtx = predicate_vtx.unwrap_or_else(|| genv.new_source(Type::Bot));
        install_local_var_write(genv, lenv, changes, var_name, type_vtx);
        return;
    }

    if let Some(arr) = pattern.as_array_pattern_node() {
        process_array_pattern(genv, lenv, changes, source, &arr, predicate_vtx);
        return;
    }

    if let Some(find) = pattern.as_find_pattern_node() {
        process_find_pattern(genv, lenv, changes, source, &find, predicate_vtx);
        return;
    }

    if let Some(hash) = pattern.as_hash_pattern_node() {
        process_hash_pattern(genv, lenv, changes, source, &hash, predicate_vtx);
        return;
    }

    // AlternationPatternNode: 1 | 2 | 3
    if let Some(alt) = pattern.as_alternation_pattern_node() {
        process_pattern(genv, lenv, changes, source, &alt.left(), predicate_vtx);
        process_pattern(genv, lenv, changes, source, &alt.right(), predicate_vtx);
        return;
    }

    // PinnedVariableNode: ^x
    if let Some(pinned) = pattern.as_pinned_variable_node() {
        install_node(genv, lenv, changes, source, &pinned.variable());
        return;
    }

    // PinnedExpressionNode: ^(expr)
    if let Some(pinned) = pattern.as_pinned_expression_node() {
        install_node(genv, lenv, changes, source, &pinned.expression());
        return;
    }

    // Literal patterns (Integer, String, etc.) - side effects only
    install_node(genv, lenv, changes, source, pattern);
}

/// Process capture pattern: Integer => x, Api::User => u
fn process_capture_pattern(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    cap: &CapturePatternNode,
) {
    let value_node = cap.value();
    let target = cap.target();
    let var_name = bytes_to_name(target.name().as_slice());

    let type_vtx = if let Some(name) = super::definitions::extract_constant_path(&value_node) {
        genv.new_source(Type::instance(&name))
    } else {
        install_node(genv, lenv, changes, source, &value_node)
            .unwrap_or_else(|| genv.new_vertex())
    };

    install_local_var_write(genv, lenv, changes, var_name, type_vtx);
}

// TODO: Remove clone 
fn type_arg_source(genv: &mut GlobalEnv, vtx: VertexId, index: usize) -> Option<VertexId> {
    let source = genv.get_source(vtx)?;
    let ty = match &source.ty {
        Type::Generic { type_args, .. } => type_args.get(index)?.clone(),
        _ => return None,
    };
    Some(genv.new_source(ty))
}

/// Process array pattern: [x, y] or [x, *rest]
fn process_array_pattern(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    arr: &ArrayPatternNode,
    predicate_vtx: Option<VertexId>,
) {
    let element_vtx = predicate_vtx.and_then(|vtx| type_arg_source(genv, vtx, 0));

    for elem in &arr.requireds() {
        process_pattern(genv, lenv, changes, source, &elem, element_vtx);
    }

    if let Some(target) = arr
        .rest()
        .and_then(|r| r.as_splat_node())
        .and_then(|s| s.expression())
        .and_then(|e| e.as_local_variable_target_node())
    {
        let var_name = bytes_to_name(target.name().as_slice());
        let rest_vtx = predicate_vtx.unwrap_or_else(|| genv.new_source(Type::array_of(Type::Bot)));
        install_local_var_write(genv, lenv, changes, var_name, rest_vtx);
    }

    for elem in &arr.posts() {
        process_pattern(genv, lenv, changes, source, &elem, element_vtx);
    }
}

/// Process hash pattern: { name:, age: }
fn process_hash_pattern(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    hash: &HashPatternNode,
    predicate_vtx: Option<VertexId>,
) {
    let value_vtx = predicate_vtx.and_then(|vtx| type_arg_source(genv, vtx, 1));

    for elem in &hash.elements() {
        if let Some(assoc) = elem.as_assoc_node() {
            process_pattern(genv, lenv, changes, source, &assoc.value(), value_vtx);
        }
    }

    if let Some(target) = hash
        .rest()
        .and_then(|r| r.as_assoc_splat_node())
        .and_then(|s| s.value())
        .and_then(|v| v.as_local_variable_target_node())
    {
        let var_name = bytes_to_name(target.name().as_slice());
        let hash_vtx = genv.new_source(Type::hash());
        install_local_var_write(genv, lenv, changes, var_name, hash_vtx);
    }
}

/// Process find pattern: [*, x, *]
fn process_find_pattern(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    find: &FindPatternNode,
    predicate_vtx: Option<VertexId>,
) {
    let element_vtx = predicate_vtx.and_then(|vtx| type_arg_source(genv, vtx, 0));
    let rest_vtx = predicate_vtx.unwrap_or_else(|| genv.new_source(Type::array_of(Type::Bot)));

    if let Some(target) = find
        .left()
        .expression()
        .and_then(|e| e.as_local_variable_target_node())
    {
        let var_name = bytes_to_name(target.name().as_slice());
        install_local_var_write(genv, lenv, changes, var_name, rest_vtx);
    }

    for elem in &find.requireds() {
        process_pattern(genv, lenv, changes, source, &elem, element_vtx);
    }

    if let Some(target) = find
        .right()
        .as_splat_node()
        .and_then(|s| s.expression())
        .and_then(|e| e.as_local_variable_target_node())
    {
        let var_name = bytes_to_name(target.name().as_slice());
        install_local_var_write(genv, lenv, changes, var_name, rest_vtx);
    }
}
