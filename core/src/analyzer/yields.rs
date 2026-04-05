//! Yield statement handling

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};

use super::install::install_node;

/// Process YieldNode: evaluate arguments for type checking, return unresolved vertex
pub(crate) fn process_yield_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    yield_node: &ruby_prism::YieldNode,
) -> Option<VertexId> {
    // TODO: Connect argument vertices to block parameter types
    if let Some(args) = yield_node.arguments() {
        for arg in args.arguments().iter() {
            install_node(genv, lenv, changes, source, &arg);
        }
    }

    Some(genv.new_vertex())
}