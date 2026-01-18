//! Literal Handlers - Processing Ruby literal values
//!
//! This module is responsible for:
//! - String, Integer, Array, Hash literals
//! - nil, true, false, Symbol literals
//! - Creating Source vertices with fixed types

use crate::env::GlobalEnv;
use crate::graph::VertexId;
use crate::types::Type;
use ruby_prism::Node;

/// Install literal nodes and return their VertexId
pub fn install_literal(genv: &mut GlobalEnv, node: &Node) -> Option<VertexId> {
    // "hello"
    if node.as_string_node().is_some() {
        return Some(genv.new_source(Type::string()));
    }

    // 42
    if node.as_integer_node().is_some() {
        return Some(genv.new_source(Type::integer()));
    }

    // [1, 2, 3]
    if node.as_array_node().is_some() {
        return Some(genv.new_source(Type::array()));
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
        return Some(genv.new_source(Type::Instance {
            class_name: "TrueClass".to_string(),
        }));
    }

    // false
    if node.as_false_node().is_some() {
        return Some(genv.new_source(Type::Instance {
            class_name: "FalseClass".to_string(),
        }));
    }

    // :symbol
    if node.as_symbol_node().is_some() {
        return Some(genv.new_source(Type::Instance {
            class_name: "Symbol".to_string(),
        }));
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
