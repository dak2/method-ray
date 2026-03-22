//! Literal Handlers - Processing Ruby literal values
//!
//! This module is responsible for:
//! - String, Integer, Float, Regexp literals
//! - nil, true, false, Symbol literals
//! - Array, Hash, Range literals with element type inference
//! - Creating Source vertices with fixed types

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};
use crate::types::Type;
use ruby_prism::Node;
use std::collections::HashSet;

/// Install literal node (including complex types: Array, Hash, Range)
pub(crate) fn install_literal_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    node: &Node,
) -> Option<VertexId> {
    if node.as_array_node().is_some() {
        let elements: Vec<Node> = node.as_array_node().unwrap().elements().iter().collect();
        return install_array_literal_elements(genv, lenv, changes, source, elements);
    }

    if node.as_hash_node().is_some() {
        let elements: Vec<Node> = node.as_hash_node().unwrap().elements().iter().collect();
        return install_hash_literal_elements(genv, lenv, changes, source, elements);
    }

    if let Some(range_node) = node.as_range_node() {
        return install_range_literal(genv, lenv, changes, source, &range_node);
    }

    // InterpolatedStringNode: "Hello, #{expr}" → String
    if let Some(interp) = node.as_interpolated_string_node() {
        for part in &interp.parts() {
            super::install::install_node(genv, lenv, changes, source, &part);
        }
        return Some(genv.new_source(Type::string()));
    }

    // InterpolatedSymbolNode: :"hello_#{expr}" → Symbol
    if let Some(interp) = node.as_interpolated_symbol_node() {
        for part in &interp.parts() {
            super::install::install_node(genv, lenv, changes, source, &part);
        }
        return Some(genv.new_source(Type::symbol()));
    }

    // InterpolatedRegularExpressionNode: /hello #{expr}/ → Regexp
    if let Some(interp) = node.as_interpolated_regular_expression_node() {
        for part in &interp.parts() {
            super::install::install_node(genv, lenv, changes, source, &part);
        }
        return Some(genv.new_source(Type::regexp()));
    }

    install_simple_literal(genv, node)
}

/// Install simple literal nodes (String, Integer, Float, nil, true, false, Symbol, Regexp)
fn install_simple_literal(genv: &mut GlobalEnv, node: &Node) -> Option<VertexId> {
    if node.as_string_node().is_some() {
        return Some(genv.new_source(Type::string()));
    }
    if node.as_integer_node().is_some() {
        return Some(genv.new_source(Type::integer()));
    }
    if node.as_float_node().is_some() {
        return Some(genv.new_source(Type::float()));
    }
    if node.as_nil_node().is_some() {
        return Some(genv.new_source(Type::Nil));
    }
    if node.as_true_node().is_some() {
        return Some(genv.new_source(Type::instance("TrueClass")));
    }
    if node.as_false_node().is_some() {
        return Some(genv.new_source(Type::instance("FalseClass")));
    }
    if node.as_symbol_node().is_some() {
        return Some(genv.new_source(Type::symbol()));
    }
    if node.as_regular_expression_node().is_some() {
        return Some(genv.new_source(Type::regexp()));
    }
    None
}

/// Install array literal with element type inference
fn install_array_literal_elements(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    elements: Vec<Node>,
) -> Option<VertexId> {
    if elements.is_empty() {
        return Some(genv.new_source(Type::array()));
    }

    let mut element_types: HashSet<Type> = HashSet::new();

    for element in &elements {
        if let Some(vtx) = super::install::install_node(genv, lenv, changes, source, element) {
            if let Some(src) = genv.get_source(vtx) {
                element_types.insert(src.ty.clone());
            } else if let Some(vertex) = genv.get_vertex(vtx) {
                for ty in vertex.types.keys() {
                    element_types.insert(ty.clone());
                }
            }
        }
    }

    let array_type = if element_types.is_empty() {
        Type::array()
    } else {
        let types_vec: Vec<Type> = element_types.into_iter().collect();
        Type::array_of(Type::union_of(types_vec))
    };

    Some(genv.new_source(array_type))
}

/// Install hash literal with key/value type inference
fn install_hash_literal_elements(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    elements: Vec<Node>,
) -> Option<VertexId> {
    if elements.is_empty() {
        return Some(genv.new_source(Type::hash()));
    }

    let mut key_types: HashSet<Type> = HashSet::new();
    let mut value_types: HashSet<Type> = HashSet::new();

    for element in &elements {
        if let Some(assoc_node) = element.as_assoc_node() {
            if let Some(key_vtx) =
                super::install::install_node(genv, lenv, changes, source, &assoc_node.key())
            {
                if let Some(src) = genv.get_source(key_vtx) {
                    key_types.insert(src.ty.clone());
                } else if let Some(vertex) = genv.get_vertex(key_vtx) {
                    for ty in vertex.types.keys() {
                        key_types.insert(ty.clone());
                    }
                }
            }

            if let Some(value_vtx) =
                super::install::install_node(genv, lenv, changes, source, &assoc_node.value())
            {
                if let Some(src) = genv.get_source(value_vtx) {
                    value_types.insert(src.ty.clone());
                } else if let Some(vertex) = genv.get_vertex(value_vtx) {
                    for ty in vertex.types.keys() {
                        value_types.insert(ty.clone());
                    }
                }
            }
        }
    }

    let hash_type = if key_types.is_empty() || value_types.is_empty() {
        Type::hash()
    } else {
        let key_type = Type::union_of(key_types.into_iter().collect());
        let value_type = Type::union_of(value_types.into_iter().collect());
        Type::hash_of(key_type, value_type)
    };

    Some(genv.new_source(hash_type))
}

/// Install range literal with element type inference
fn install_range_literal(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    range_node: &ruby_prism::RangeNode,
) -> Option<VertexId> {
    let element_type = if let Some(left) = range_node.left() {
        infer_range_element_type(genv, lenv, changes, source, &left)
    } else if let Some(right) = range_node.right() {
        infer_range_element_type(genv, lenv, changes, source, &right)
    } else {
        None
    };

    let range_type = match element_type {
        Some(ty) => Type::range_of(ty),
        None => Type::range(),
    };

    Some(genv.new_source(range_type))
}

/// Infer element type from a range endpoint node
fn infer_range_element_type(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    node: &Node,
) -> Option<Type> {
    if let Some(vtx) = super::install::install_node(genv, lenv, changes, source, node) {
        if let Some(src) = genv.get_source(vtx) {
            return Some(src.ty.clone());
        }
        if let Some(vertex) = genv.get_vertex(vtx) {
            if let Some(ty) = vertex.types.keys().next() {
                return Some(ty.clone());
            }
        }
    }
    None
}
