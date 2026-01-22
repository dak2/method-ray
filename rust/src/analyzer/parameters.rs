//! Parameter Handlers - Processing Ruby method/block parameters
//!
//! This module is responsible for:
//! - Extracting parameter names from DefNode
//! - Creating vertices for parameters
//! - Registering parameters as local variables in method scope

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};
use crate::types::Type;

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
pub fn install_required_parameter(genv: &mut GlobalEnv, lenv: &mut LocalEnv, name: String) -> VertexId {
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
pub fn install_optional_parameter(
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
pub fn install_rest_parameter(genv: &mut GlobalEnv, lenv: &mut LocalEnv, name: String) -> VertexId {
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
pub fn install_keyword_rest_parameter(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_required_parameter() {
        let mut genv = GlobalEnv::new();
        let mut lenv = LocalEnv::new();

        let vtx = install_required_parameter(&mut genv, &mut lenv, "name".to_string());

        // Parameter should be registered in LocalEnv
        assert_eq!(lenv.get_var("name"), Some(vtx));

        // Vertex should exist in GlobalEnv (as untyped)
        let vertex = genv.get_vertex(vtx);
        assert!(vertex.is_some());
    }

    #[test]
    fn test_install_multiple_parameters() {
        let mut genv = GlobalEnv::new();
        let mut lenv = LocalEnv::new();

        let vtx_a = install_required_parameter(&mut genv, &mut lenv, "a".to_string());
        let vtx_b = install_required_parameter(&mut genv, &mut lenv, "b".to_string());
        let vtx_c = install_required_parameter(&mut genv, &mut lenv, "c".to_string());

        // All parameters should be registered
        assert_eq!(lenv.get_var("a"), Some(vtx_a));
        assert_eq!(lenv.get_var("b"), Some(vtx_b));
        assert_eq!(lenv.get_var("c"), Some(vtx_c));

        // All vertices should be different
        assert_ne!(vtx_a, vtx_b);
        assert_ne!(vtx_b, vtx_c);
        assert_ne!(vtx_a, vtx_c);
    }
}
