//! Parameter Handlers - Processing Ruby method/block parameters
//!
//! This module is responsible for:
//! - Extracting parameter names from DefNode
//! - Creating vertices for parameters
//! - Registering parameters as local variables in method scope

use std::collections::HashMap;

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};
use crate::types::Type;

use super::bytes_to_name;

/// Install a required parameter as a local variable
///
/// Required parameters start with Bot (untyped) type since we don't know
/// what type will be passed at call sites.
///
/// # Example
/// ```ruby
/// def greet(name)  # 'name' is a required parameter
///   name.upcase
/// end
/// ```
pub(crate) fn install_required_parameter(genv: &mut GlobalEnv, lenv: &mut LocalEnv, name: String) -> VertexId {
    // Create a vertex for the parameter (starts as Bot/untyped)
    let param_vtx = genv.new_vertex();

    // Register in LocalEnv for variable lookup
    lenv.new_var(name, param_vtx);

    param_vtx
}

/// Install an optional parameter with a default value
///
/// The parameter's type is inferred from the default value expression.
///
/// # Example
/// ```ruby
/// def greet(name = "World")  # 'name' has type String from default
///   name.upcase
/// end
/// ```
pub(crate) fn install_optional_parameter(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    _changes: &mut ChangeSet,
    name: String,
    default_value_vtx: VertexId,
) -> VertexId {
    // Create a vertex for the parameter
    let param_vtx = genv.new_vertex();

    // Connect default value to parameter vertex for type inference
    // Use genv.add_edge directly so the type is immediately propagated
    // before the method body is processed
    genv.add_edge(default_value_vtx, param_vtx);

    // Register in LocalEnv for variable lookup
    lenv.new_var(name, param_vtx);

    param_vtx
}

/// Install a rest parameter (*args) as a local variable with Array type
///
/// Rest parameters collect all remaining arguments into an Array.
///
/// # Example
/// ```ruby
/// def collect(*items)  # 'items' has type Array
///   items.first
/// end
/// ```
pub(crate) fn install_rest_parameter(genv: &mut GlobalEnv, lenv: &mut LocalEnv, name: String) -> VertexId {
    // Create a vertex for the parameter
    let param_vtx = genv.new_vertex();

    // Rest parameters are always Arrays
    let array_src = genv.new_source(Type::array());
    genv.add_edge(array_src, param_vtx);

    // Register in LocalEnv for variable lookup
    lenv.new_var(name, param_vtx);

    param_vtx
}

/// Install a keyword rest parameter (**kwargs) as a local variable with Hash type
///
/// Keyword rest parameters collect all remaining keyword arguments into a Hash.
///
/// # Example
/// ```ruby
/// def configure(**options)  # 'options' has type Hash
///   options[:debug]
/// end
/// ```
pub(crate) fn install_keyword_rest_parameter(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    name: String,
) -> VertexId {
    // Create a vertex for the parameter
    let param_vtx = genv.new_vertex();

    // Keyword rest parameters are always Hashes
    let hash_src = genv.new_source(Type::hash());
    genv.add_edge(hash_src, param_vtx);

    // Register in LocalEnv for variable lookup
    lenv.new_var(name, param_vtx);

    param_vtx
}

/// Install method parameters as local variables
///
/// Returns a tuple of:
/// - Vec<VertexId>: positional parameter vertices (required and optional)
/// - HashMap<String, VertexId>: keyword parameter vertices (name → vertex)
pub(crate) fn install_parameters(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    params_node: &ruby_prism::ParametersNode,
) -> (Vec<VertexId>, HashMap<String, VertexId>) {
    let mut param_vtxs = Vec::new();
    let mut keyword_param_vtxs: HashMap<String, VertexId> = HashMap::new();

    // Required parameters: def foo(a, b)
    for node in params_node.requireds().iter() {
        if let Some(req_param) = node.as_required_parameter_node() {
            let name = bytes_to_name(req_param.name().as_slice());
            let vtx = install_required_parameter(genv, lenv, name);
            param_vtxs.push(vtx);
        }
    }

    // Optional parameters: def foo(a = 1, b = "hello")
    for node in params_node.optionals().iter() {
        if let Some(opt_param) = node.as_optional_parameter_node() {
            let name = bytes_to_name(opt_param.name().as_slice());
            let default_value = opt_param.value();

            let vtx = if let Some(default_vtx) =
                super::install::install_node(genv, lenv, changes, source, &default_value)
            {
                install_optional_parameter(genv, lenv, changes, name, default_vtx)
            } else {
                install_required_parameter(genv, lenv, name)
            };
            param_vtxs.push(vtx);
        }
    }

    // Rest parameter: def foo(*args)
    // Not included in param_vtxs (variadic args need special handling)
    if let Some(rest_node) = params_node.rest() {
        if let Some(rest_param) = rest_node.as_rest_parameter_node() {
            if let Some(name_id) = rest_param.name() {
                let name = bytes_to_name(name_id.as_slice());
                install_rest_parameter(genv, lenv, name);
            }
        }
    }

    // Keyword parameters: def foo(name:, age: 0)
    // Reuses install_required_parameter / install_optional_parameter
    // since the logic is identical for positional and keyword parameters.
    for node in params_node.keywords().iter() {
        if let Some(req_kw) = node.as_required_keyword_parameter_node() {
            let name = bytes_to_name(req_kw.name().as_slice());
            let vtx = install_required_parameter(genv, lenv, name.clone());
            keyword_param_vtxs.insert(name, vtx);
        } else if let Some(opt_kw) = node.as_optional_keyword_parameter_node() {
            let name = bytes_to_name(opt_kw.name().as_slice());
            let default_value = opt_kw.value();
            let vtx = if let Some(default_vtx) =
                super::install::install_node(genv, lenv, changes, source, &default_value)
            {
                install_optional_parameter(genv, lenv, changes, name.clone(), default_vtx)
            } else {
                install_required_parameter(genv, lenv, name.clone())
            };
            keyword_param_vtxs.insert(name, vtx);
        }
    }

    // Keyword rest parameter: def foo(**kwargs)
    // Not included in keyword_param_vtxs (collects all remaining keywords)
    if let Some(kwrest_node) = params_node.keyword_rest() {
        if let Some(kwrest_param) = kwrest_node.as_keyword_rest_parameter_node() {
            if let Some(name_id) = kwrest_param.name() {
                let name = bytes_to_name(name_id.as_slice());
                install_keyword_rest_parameter(genv, lenv, name);
            }
        }
    }

    (param_vtxs, keyword_param_vtxs)
}
