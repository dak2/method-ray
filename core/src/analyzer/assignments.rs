//! Multiple Assignment Handlers - Processing Ruby multiple assignment
//!
//! Supports: ArrayNode RHS with 1:1 mapping, LHS > RHS nil fill,
//! splat targets (*rest) as Array type, and basic single-expression RHS decomposition.
//! TODO: Support RHS as method return value decomposition (requires graph lazy resolution)

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};
use crate::types::Type;

use super::bytes_to_name;
use super::variables::install_local_var_write;

/// Install an RHS node and assign it to a named local variable.
/// Falls back to an untyped vertex when `install_node` returns `None`.
fn install_target(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    var_name: String,
    rhs_node: &ruby_prism::Node,
) -> VertexId {
    if let Some(rv) = super::install::install_node(genv, lenv, changes, source, rhs_node) {
        install_local_var_write(genv, lenv, changes, var_name, rv)
    } else {
        let vtx = genv.new_vertex();
        lenv.new_var(var_name, vtx);
        vtx
    }
}

/// Assign `Type::Nil` to a named local variable.
fn install_nil_target(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    var_name: String,
) -> VertexId {
    let nil_src = genv.new_source(Type::Nil);
    install_local_var_write(genv, lenv, changes, var_name, nil_src)
}

/// Extract the splat target variable name from a `node.rest()` result.
fn splat_var_name(rest_node: &ruby_prism::Node) -> Option<String> {
    let splat = rest_node.as_splat_node()?;
    let expr = splat.expression()?;
    let target = expr.as_local_variable_target_node()?;
    Some(bytes_to_name(target.name().as_slice()))
}

/// Collect unique types from a slice of elements, returning element type for Array[T].
/// Only nodes that resolve to a Source (fixed type) contribute; Vertex-type nodes
/// are excluded (known limitation — requires lazy resolution for method return values).
fn collect_element_type(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    elements: &[ruby_prism::Node],
) -> Type {
    let mut types: Vec<Type> = Vec::new();
    for elem in elements {
        if let Some(vtx) = super::install::install_node(genv, lenv, changes, source, elem) {
            if let Some(src) = genv.get_source(vtx) {
                if !types.contains(&src.ty) {
                    types.push(src.ty.clone());
                }
            }
        }
    }
    if types.is_empty() {
        Type::Bot
    } else {
        Type::union_of(types)
    }
}

/// Process multiple assignment node (e.g., `a, b = 1, "hello"`)
pub(crate) fn process_multi_write_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    node: &ruby_prism::MultiWriteNode,
) -> Option<VertexId> {
    let value = node.value();
    let mut last_vtx = None;

    if let Some(array_node) = value.as_array_node() {
        let lefts = node.lefts();
        let elements: Vec<_> = array_node.elements().iter().collect();
        let total = elements.len();
        let lefts_count = lefts.len();
        let rights = node.rights();
        let rights_count = rights.len();

        // Phase 1: Left targets — assign from start, nil for missing RHS
        for (i, target) in lefts.iter().enumerate() {
            if let Some(target_node) = target.as_local_variable_target_node() {
                let var_name = bytes_to_name(target_node.name().as_slice());
                if i < total {
                    last_vtx = Some(install_target(
                        genv,
                        lenv,
                        changes,
                        source,
                        var_name,
                        &elements[i],
                    ));
                } else {
                    last_vtx = Some(install_nil_target(genv, lenv, changes, var_name));
                }
            }
        }

        // Phase 2: Splat target (*rest) — collect middle elements (after lefts, before rights) into Array[T]
        if let Some(rest_node) = node.rest() {
            if let Some(var_name) = splat_var_name(&rest_node) {
                let splat_start = lefts_count;
                let splat_end = total.saturating_sub(rights_count);
                let splat_elements = if splat_start < splat_end {
                    &elements[splat_start..splat_end]
                } else {
                    &elements[0..0]
                };
                let element_type = collect_element_type(
                    genv,
                    lenv,
                    changes,
                    source,
                    splat_elements,
                );
                let array_src = genv.new_source(Type::array_of(element_type));
                last_vtx = Some(install_local_var_write(
                    genv, lenv, changes, var_name, array_src,
                ));
            }
        }

        // Phase 3: Right targets — assigned from end of elements, nil if overlapping with lefts
        for (i, target) in rights.iter().enumerate() {
            if let Some(target_node) = target.as_local_variable_target_node() {
                let var_name = bytes_to_name(target_node.name().as_slice());
                let signed_idx = total as isize - rights_count as isize + i as isize;
                if signed_idx >= lefts_count as isize && (signed_idx as usize) < total {
                    last_vtx = Some(install_target(
                        genv,
                        lenv,
                        changes,
                        source,
                        var_name,
                        &elements[signed_idx as usize],
                    ));
                } else {
                    last_vtx = Some(install_nil_target(genv, lenv, changes, var_name));
                }
            }
        }
    } else {
        // RHS is a single expression (not comma-separated)
        let rhs_vtx = super::install::install_node(genv, lenv, changes, source, &value);

        let rhs_type = rhs_vtx
            .and_then(|vtx| genv.get_source(vtx))
            .map(|src| src.ty.clone());

        // If RHS is Array[T], each target gets T; otherwise first target gets RHS, rest get nil
        let element_type = rhs_type
            .as_ref()
            .and_then(|ty| ty.type_args())
            .and_then(|args| args.first().cloned());

        for (i, target) in node.lefts().iter().enumerate() {
            if let Some(target_node) = target.as_local_variable_target_node() {
                let var_name = bytes_to_name(target_node.name().as_slice());
                if let Some(ref elem_ty) = element_type {
                    let src = genv.new_source(elem_ty.clone());
                    last_vtx = Some(install_local_var_write(genv, lenv, changes, var_name, src));
                } else if i == 0 {
                    if let Some(rv) = rhs_vtx {
                        last_vtx = Some(install_local_var_write(genv, lenv, changes, var_name, rv));
                    } else {
                        let vtx = genv.new_vertex();
                        lenv.new_var(var_name, vtx);
                        last_vtx = Some(vtx);
                    }
                } else if rhs_type.is_some() {
                    last_vtx = Some(install_nil_target(genv, lenv, changes, var_name));
                } else {
                    let vtx = genv.new_vertex();
                    lenv.new_var(var_name, vtx);
                    last_vtx = Some(vtx);
                }
            }
        }

        // Splat in single-expression RHS
        if let Some(rest_node) = node.rest() {
            if let Some(var_name) = splat_var_name(&rest_node) {
                let elem_ty = element_type.unwrap_or(Type::Bot);
                let array_src = genv.new_source(Type::array_of(elem_ty));
                last_vtx = Some(install_local_var_write(
                    genv, lenv, changes, var_name, array_src,
                ));
            }
        }

        // Right targets in single-expression RHS → nil
        for target in node.rights().iter() {
            if let Some(target_node) = target.as_local_variable_target_node() {
                let var_name = bytes_to_name(target_node.name().as_slice());
                last_vtx = Some(install_nil_target(genv, lenv, changes, var_name));
            }
        }
    }

    last_vtx
}
