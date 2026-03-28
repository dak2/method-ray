//! AST Installer - AST traversal and graph construction
//!
//! This module is responsible for:
//! - Traversing the Ruby AST (Abstract Syntax Tree)
//! - Coordinating the graph construction process

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};
use ruby_prism::Node;

use super::assignments::process_multi_write_node;
use super::blocks::process_block_node;
use super::conditionals::{process_case_match_node, process_case_node, process_if_node, process_unless_node};
use super::definitions::{process_class_node, process_def_node, process_module_node};
use super::exceptions::{process_begin_node, process_rescue_modifier_node};
use super::dispatch::{dispatch_needs_child, dispatch_simple, process_needs_child, DispatchResult};
use super::literals::install_literal_node;
use super::loops::{process_for_node, process_until_node, process_while_node};
use super::operators::{process_and_node, process_or_node};
use super::parentheses::process_parentheses_node;
use super::returns::process_return_node;
use super::super_calls;

/// Build graph from AST (public API wrapper)
pub struct AstInstaller<'a> {
    genv: &'a mut GlobalEnv,
    lenv: &'a mut LocalEnv,
    changes: ChangeSet,
    source: &'a str,
}

impl<'a> AstInstaller<'a> {
    pub fn new(genv: &'a mut GlobalEnv, lenv: &'a mut LocalEnv, source: &'a str) -> Self {
        Self {
            genv,
            lenv,
            changes: ChangeSet::new(),
            source,
        }
    }

    pub fn install_node(&mut self, node: &Node) -> Option<VertexId> {
        install_node(self.genv, self.lenv, &mut self.changes, self.source, node)
    }

    pub fn finish(self) {
        self.genv.apply_changes(self.changes);
        self.genv.run_all();
    }
}

/// Install node (returns Vertex ID)
pub(crate) fn install_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    node: &Node,
) -> Option<VertexId> {
    if let Some(class_node) = node.as_class_node() {
        return process_class_node(genv, lenv, changes, source, &class_node);
    }

    if let Some(module_node) = node.as_module_node() {
        return process_module_node(genv, lenv, changes, source, &module_node);
    }

    if let Some(def_node) = node.as_def_node() {
        return process_def_node(genv, lenv, changes, source, &def_node);
    }

    if let Some(block_node) = node.as_block_node() {
        return process_block_node(genv, lenv, changes, source, &block_node);
    }

    if let Some(if_node) = node.as_if_node() {
        return process_if_node(genv, lenv, changes, source, &if_node);
    }
    if let Some(unless_node) = node.as_unless_node() {
        return process_unless_node(genv, lenv, changes, source, &unless_node);
    }
    if let Some(case_node) = node.as_case_node() {
        return process_case_node(genv, lenv, changes, source, &case_node);
    }
    if let Some(case_match_node) = node.as_case_match_node() {
        return process_case_match_node(genv, lenv, changes, source, &case_match_node);
    }

    if let Some(begin_node) = node.as_begin_node() {
        return process_begin_node(genv, lenv, changes, source, &begin_node);
    }
    if let Some(rescue_modifier) = node.as_rescue_modifier_node() {
        return process_rescue_modifier_node(genv, lenv, changes, source, &rescue_modifier);
    }

    // SuperNode: super(args) — explicit arguments
    if let Some(super_node) = node.as_super_node() {
        return super_calls::process_super_node(genv, lenv, changes, source, &super_node);
    }
    // ForwardingSuperNode: super — implicit argument forwarding
    if let Some(fwd_super_node) = node.as_forwarding_super_node() {
        return super_calls::process_forwarding_super_node(
            genv, lenv, changes, source, &fwd_super_node,
        );
    }

    if let Some(while_node) = node.as_while_node() {
        return process_while_node(genv, lenv, changes, source, &while_node);
    }
    if let Some(until_node) = node.as_until_node() {
        return process_until_node(genv, lenv, changes, source, &until_node);
    }
    if let Some(for_node) = node.as_for_node() {
        return process_for_node(genv, lenv, changes, source, &for_node);
    }

    if let Some(paren_node) = node.as_parentheses_node() {
        return process_parentheses_node(genv, lenv, changes, source, &paren_node);
    }

    if let Some(return_node) = node.as_return_node() {
        return process_return_node(genv, lenv, changes, source, &return_node);
    }

    if let Some(and_node) = node.as_and_node() {
        return process_and_node(genv, lenv, changes, source, &and_node);
    }
    if let Some(or_node) = node.as_or_node() {
        return process_or_node(genv, lenv, changes, source, &or_node);
    }

    if let Some(multi_write) = node.as_multi_write_node() {
        return process_multi_write_node(genv, lenv, changes, source, &multi_write);
    }

    match dispatch_simple(genv, lenv, node) {
        DispatchResult::Vertex(vtx) => return Some(vtx),
        DispatchResult::NotHandled => {}
    }

    if let Some(vtx) = install_literal_node(genv, lenv, changes, source, node) {
        return Some(vtx);
    }

    if let Some(kind) = dispatch_needs_child(node, source) {
        return process_needs_child(genv, lenv, changes, source, kind);
    }

    None
}

/// Process multiple statements (returns last expression's VertexId)
pub(crate) fn install_statements(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    statements: &ruby_prism::StatementsNode,
) -> Option<VertexId> {
    let mut last_vtx = None;
    for stmt in &statements.body() {
        last_vtx = install_node(genv, lenv, changes, source, &stmt);
    }
    last_vtx
}
