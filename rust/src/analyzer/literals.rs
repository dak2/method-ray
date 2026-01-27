//! Literal Handlers - Processing Ruby literal values
//!
//! This module is responsible for:
//! - String, Integer, Hash literals
//! - nil, true, false, Symbol literals
//! - Creating Source vertices with fixed types
//!
//! Note: Array literals are handled in install.rs for element type inference

use crate::env::GlobalEnv;
use crate::graph::VertexId;
use crate::types::Type;
use ruby_prism::Node;

/// Install literal nodes and return their VertexId
///
/// Note: Array literals are NOT handled here because they require
/// child processing for element type inference. See install.rs.
pub fn install_literal(genv: &mut GlobalEnv, node: &Node) -> Option<VertexId> {
    // "hello"
    if node.as_string_node().is_some() {
        return Some(genv.new_source(Type::string()));
    }

    // 42
    if node.as_integer_node().is_some() {
        return Some(genv.new_source(Type::integer()));
    }

    // {a: 1}
    if node.as_hash_node().is_some() {
        return Some(genv.new_source(Type::hash()));
    }

    // nil
    if node.as_nil_node().is_some() {
        return Some(genv.new_source(Type::Nil));
    }

    // true
    if node.as_true_node().is_some() {
        return Some(genv.new_source(Type::instance("TrueClass")));
    }

    // false
    if node.as_false_node().is_some() {
        return Some(genv.new_source(Type::instance("FalseClass")));
    }

    // :symbol
    if node.as_symbol_node().is_some() {
        return Some(genv.new_source(Type::symbol()));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_string_literal() {
        let mut genv = GlobalEnv::new();

        // Create a mock string node - we test via integration instead
        // Unit test just verifies the type creation
        let vtx = genv.new_source(Type::string());
        assert_eq!(genv.get_source(vtx).unwrap().ty.show(), "String");
    }

    #[test]
    fn test_install_integer_literal() {
        let mut genv = GlobalEnv::new();

        let vtx = genv.new_source(Type::integer());
        assert_eq!(genv.get_source(vtx).unwrap().ty.show(), "Integer");
    }
}
