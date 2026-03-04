//! Block Handlers - Processing Ruby blocks
//!
//! This module is responsible for:
//! - Processing BlockNode (e.g., `{ |x| x.to_s }` or `do |x| x.to_s end`)
//! - Registering block parameters as local variables
//! - Managing block scope

use crate::env::{GlobalEnv, LocalEnv, ScopeKind};
use crate::graph::{ChangeSet, VertexId};

use super::bytes_to_name;
use super::parameters::{install_optional_parameter, install_required_parameter, install_rest_parameter};

/// Process block node
pub(crate) fn process_block_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    block_node: &ruby_prism::BlockNode,
) -> Option<VertexId> {
    process_block_node_with_params(genv, lenv, changes, source, block_node);
    None
}

/// Process block node and return block parameter vertex IDs
pub(crate) fn process_block_node_with_params(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    block_node: &ruby_prism::BlockNode,
) -> Vec<VertexId> {
    enter_block_scope(genv);

    let mut param_vtxs = Vec::new();

    if let Some(params_node) = block_node.parameters() {
        if let Some(block_params) = params_node.as_block_parameters_node() {
            param_vtxs =
                install_block_parameters_with_vtxs(genv, lenv, changes, source, &block_params);
        }
    }

    if let Some(body) = block_node.body() {
        if let Some(statements) = body.as_statements_node() {
            super::install::install_statements(genv, lenv, changes, source, &statements);
        } else {
            super::install::install_node(genv, lenv, changes, source, &body);
        }
    }

    exit_block_scope(genv);

    param_vtxs
}

/// Install block parameters and return their vertex IDs
fn install_block_parameters_with_vtxs(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    block_params: &ruby_prism::BlockParametersNode,
) -> Vec<VertexId> {
    let mut vtxs = Vec::new();

    if let Some(params) = block_params.parameters() {
        // Required parameters (most common in blocks)
        for node in params.requireds().iter() {
            if let Some(req_param) = node.as_required_parameter_node() {
                let name = bytes_to_name(req_param.name().as_slice());
                let vtx = install_block_parameter(genv, lenv, name);
                vtxs.push(vtx);
            }
        }

        // Optional parameters: { |x = 1| ... }
        for node in params.optionals().iter() {
            if let Some(opt_param) = node.as_optional_parameter_node() {
                let name = bytes_to_name(opt_param.name().as_slice());
                let default_value = opt_param.value();

                if let Some(default_vtx) =
                    super::install::install_node(genv, lenv, changes, source, &default_value)
                {
                    let vtx =
                        install_optional_parameter(genv, lenv, changes, name, default_vtx);
                    vtxs.push(vtx);
                } else {
                    let vtx = install_block_parameter(genv, lenv, name);
                    vtxs.push(vtx);
                }
            }
        }

        // Rest parameter: { |*args| ... }
        if let Some(rest_node) = params.rest() {
            if let Some(rest_param) = rest_node.as_rest_parameter_node() {
                if let Some(name_id) = rest_param.name() {
                    let name = bytes_to_name(name_id.as_slice());
                    let vtx = install_rest_parameter(genv, lenv, name);
                    vtxs.push(vtx);
                }
            }
        }
    }

    vtxs
}

/// Enter a new block scope
fn enter_block_scope(genv: &mut GlobalEnv) {
    let block_scope_id = genv.scope_manager.new_scope(ScopeKind::Block);
    genv.scope_manager.enter_scope(block_scope_id);
}

/// Exit the current block scope
fn exit_block_scope(genv: &mut GlobalEnv) {
    genv.scope_manager.exit_scope();
}

/// Install block parameter as a local variable (Bot type)
fn install_block_parameter(genv: &mut GlobalEnv, lenv: &mut LocalEnv, name: String) -> VertexId {
    install_required_parameter(genv, lenv, name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enter_exit_block_scope() {
        let mut genv = GlobalEnv::new();

        let initial_scope_id = genv.scope_manager.current_scope().id;

        enter_block_scope(&mut genv);
        let block_scope_id = genv.scope_manager.current_scope().id;

        assert_ne!(initial_scope_id, block_scope_id);

        exit_block_scope(&mut genv);

        assert_eq!(genv.scope_manager.current_scope().id, initial_scope_id);
    }

    #[test]
    fn test_install_block_parameter() {
        let mut genv = GlobalEnv::new();
        let mut lenv = LocalEnv::new();

        enter_block_scope(&mut genv);

        let vtx = install_block_parameter(&mut genv, &mut lenv, "x".to_string());

        assert_eq!(lenv.get_var("x"), Some(vtx));

        exit_block_scope(&mut genv);
    }

    #[test]
    fn test_block_inherits_parent_scope_vars() {
        let mut genv = GlobalEnv::new();

        genv.scope_manager
            .current_scope_mut()
            .set_local_var("outer".to_string(), VertexId(100));

        enter_block_scope(&mut genv);

        assert_eq!(genv.scope_manager.lookup_var("outer"), Some(VertexId(100)));

        exit_block_scope(&mut genv);
    }
}
