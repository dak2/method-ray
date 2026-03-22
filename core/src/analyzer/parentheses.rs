//! Parentheses - pass-through type propagation for parenthesized expressions

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};

use super::install::{install_node, install_statements};

/// Process ParenthesesNode: propagate inner expression's type
pub(crate) fn process_parentheses_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    paren_node: &ruby_prism::ParenthesesNode,
) -> Option<VertexId> {
    let body = paren_node.body()?;

    if let Some(stmts) = body.as_statements_node() {
        // (expr1; expr2) → process all, return last expression's type
        install_statements(genv, lenv, changes, source, &stmts)
    } else {
        // (expr) → propagate inner expression's type directly
        install_node(genv, lenv, changes, source, &body)
    }
}
