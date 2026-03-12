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

#[cfg(test)]
mod tests {
    use crate::analyzer::install::AstInstaller;
    use crate::env::{GlobalEnv, LocalEnv};
    use crate::graph::VertexId;
    use crate::parser::ParseSession;

    fn analyze(source: &str) -> (GlobalEnv, LocalEnv) {
        let session = ParseSession::new();
        let parse_result = session.parse_source(source, "test.rb").unwrap();
        let root = parse_result.node();
        let program = root.as_program_node().unwrap();

        let mut genv = GlobalEnv::new();
        let mut lenv = LocalEnv::new();

        let mut installer = AstInstaller::new(&mut genv, &mut lenv, source);
        for stmt in &program.statements().body() {
            installer.install_node(&stmt);
        }
        installer.finish();

        (genv, lenv)
    }

    fn get_type_show(genv: &GlobalEnv, vtx: VertexId) -> String {
        if let Some(vertex) = genv.get_vertex(vtx) {
            vertex.show()
        } else if let Some(source) = genv.get_source(vtx) {
            source.ty.show()
        } else {
            panic!("vertex {:?} not found as either Vertex or Source", vtx);
        }
    }

    #[test]
    fn test_multi_write_integer_and_string() {
        let source = r#"a, b = 1, "hello""#;
        let (genv, lenv) = analyze(source);

        let a_vtx = lenv.get_var("a").expect("a should be registered");
        assert_eq!(get_type_show(&genv, a_vtx), "Integer");

        let b_vtx = lenv.get_var("b").expect("b should be registered");
        assert_eq!(get_type_show(&genv, b_vtx), "String");
    }

    #[test]
    fn test_multi_write_all_integer() {
        let source = "a, b, c = 1, 2, 3";
        let (genv, lenv) = analyze(source);

        let a_vtx = lenv.get_var("a").expect("a should be registered");
        assert_eq!(get_type_show(&genv, a_vtx), "Integer");

        let b_vtx = lenv.get_var("b").expect("b should be registered");
        assert_eq!(get_type_show(&genv, b_vtx), "Integer");

        let c_vtx = lenv.get_var("c").expect("c should be registered");
        assert_eq!(get_type_show(&genv, c_vtx), "Integer");
    }

    #[test]
    fn test_multi_write_variable_reference_after_assignment() {
        let source = r#"
a, b = 1, "hello"
x = a
"#;
        let (genv, lenv) = analyze(source);

        let x_vtx = lenv.get_var("x").expect("x should be registered");
        assert_eq!(get_type_show(&genv, x_vtx), "Integer");
    }

    #[test]
    fn test_multi_write_lhs_longer_than_rhs() {
        let source = "a, b, c = 1, 2";
        let (genv, lenv) = analyze(source);

        let a_vtx = lenv.get_var("a").expect("a should be registered");
        assert_eq!(get_type_show(&genv, a_vtx), "Integer");

        let b_vtx = lenv.get_var("b").expect("b should be registered");
        assert_eq!(get_type_show(&genv, b_vtx), "Integer");

        let c_vtx = lenv.get_var("c").expect("c should be registered with nil");
        assert_eq!(get_type_show(&genv, c_vtx), "nil");
    }

    #[test]
    fn test_multi_write_does_not_panic_on_non_array_rhs() {
        let source = "a, b = some_expr";
        let (_, lenv) = analyze(source);

        // Variables should be registered (untyped) without panic
        assert!(lenv.get_var("a").is_some(), "a should be registered");
        assert!(lenv.get_var("b").is_some(), "b should be registered");
    }

    #[test]
    fn test_multi_write_splat_basic() {
        let source = "first, *rest = 1, 2, 3";
        let (genv, lenv) = analyze(source);

        let first_vtx = lenv.get_var("first").expect("first should be registered");
        assert_eq!(get_type_show(&genv, first_vtx), "Integer");

        let rest_vtx = lenv.get_var("rest").expect("rest should be registered");
        assert_eq!(get_type_show(&genv, rest_vtx), "Array[Integer]");
    }

    #[test]
    fn test_multi_write_splat_mixed_types() {
        let source = r#"first, *rest = 1, "hello", :sym"#;
        let (genv, lenv) = analyze(source);

        let first_vtx = lenv.get_var("first").expect("first should be registered");
        assert_eq!(get_type_show(&genv, first_vtx), "Integer");

        let rest_vtx = lenv.get_var("rest").expect("rest should be registered");
        let type_str = get_type_show(&genv, rest_vtx);
        assert!(
            type_str.contains("Array"),
            "should be Array type: {}",
            type_str
        );
        assert!(
            type_str.contains("String"),
            "should contain String: {}",
            type_str
        );
        assert!(
            type_str.contains("Symbol"),
            "should contain Symbol: {}",
            type_str
        );
    }

    #[test]
    fn test_multi_write_splat_empty() {
        let source = "first, *rest = 1";
        let (genv, lenv) = analyze(source);

        let first_vtx = lenv.get_var("first").expect("first should be registered");
        assert_eq!(get_type_show(&genv, first_vtx), "Integer");

        let rest_vtx = lenv.get_var("rest").expect("rest should be registered");
        assert_eq!(get_type_show(&genv, rest_vtx), "Array[untyped]");
    }

    #[test]
    fn test_multi_write_splat_with_rights() {
        let source = "first, *rest, last = 1, 2, 3, 4";
        let (genv, lenv) = analyze(source);

        let first_vtx = lenv.get_var("first").expect("first should be registered");
        assert_eq!(get_type_show(&genv, first_vtx), "Integer");

        let rest_vtx = lenv.get_var("rest").expect("rest should be registered");
        assert_eq!(get_type_show(&genv, rest_vtx), "Array[Integer]");

        let last_vtx = lenv.get_var("last").expect("last should be registered");
        assert_eq!(get_type_show(&genv, last_vtx), "Integer");
    }

    #[test]
    fn test_multi_write_splat_only() {
        let source = "*all = 1, 2, 3";
        let (genv, lenv) = analyze(source);

        let all_vtx = lenv.get_var("all").expect("all should be registered");
        assert_eq!(get_type_show(&genv, all_vtx), "Array[Integer]");
    }

    #[test]
    fn test_multi_write_splat_rights_no_lefts() {
        let source = "*rest, last = 1, 2, 3";
        let (genv, lenv) = analyze(source);

        let rest_vtx = lenv.get_var("rest").expect("rest should be registered");
        assert_eq!(get_type_show(&genv, rest_vtx), "Array[Integer]");

        let last_vtx = lenv.get_var("last").expect("last should be registered");
        assert_eq!(get_type_show(&genv, last_vtx), "Integer");
    }

    #[test]
    fn test_multi_write_array_literal_rhs() {
        // Explicit array literal RHS is decomposed element-by-element (same as comma-separated form)
        let source = r#"a, b = [1, "hi"]"#;
        let (genv, lenv) = analyze(source);

        let a_vtx = lenv.get_var("a").expect("a should be registered");
        assert_eq!(get_type_show(&genv, a_vtx), "Integer");

        let b_vtx = lenv.get_var("b").expect("b should be registered");
        assert_eq!(get_type_show(&genv, b_vtx), "String");
    }

    #[test]
    fn test_multi_write_splat_lefts_exceed_rhs() {
        // Edge case: more left targets than RHS elements with splat
        let source = "a, b, c, *rest = 1, 2";
        let (genv, lenv) = analyze(source);

        let a_vtx = lenv.get_var("a").expect("a should be registered");
        assert_eq!(get_type_show(&genv, a_vtx), "Integer");

        let b_vtx = lenv.get_var("b").expect("b should be registered");
        assert_eq!(get_type_show(&genv, b_vtx), "Integer");

        let c_vtx = lenv.get_var("c").expect("c should be registered");
        assert_eq!(get_type_show(&genv, c_vtx), "nil");

        let rest_vtx = lenv.get_var("rest").expect("rest should be registered");
        assert_eq!(get_type_show(&genv, rest_vtx), "Array[untyped]");
    }

    #[test]
    fn test_multi_write_splat_with_rights_insufficient_rhs() {
        // Edge case: lefts + rights > total elements, splat between them
        let source = "a, *rest, z = 1";
        let (genv, lenv) = analyze(source);

        let a_vtx = lenv.get_var("a").expect("a should be registered");
        assert_eq!(get_type_show(&genv, a_vtx), "Integer");

        let rest_vtx = lenv.get_var("rest").expect("rest should be registered");
        assert_eq!(get_type_show(&genv, rest_vtx), "Array[untyped]");

        let z_vtx = lenv.get_var("z").expect("z should be registered");
        assert_eq!(get_type_show(&genv, z_vtx), "nil");
    }

    #[test]
    fn test_multi_write_rights_exceed_rhs() {
        // Edge case: more right targets than available elements
        let source = r#"*rest, x, y, z = "a", 1"#;
        let (genv, lenv) = analyze(source);

        let rest_vtx = lenv.get_var("rest").expect("rest should be registered");
        assert_eq!(get_type_show(&genv, rest_vtx), "Array[untyped]");

        let x_vtx = lenv.get_var("x").expect("x should be registered");
        assert_eq!(get_type_show(&genv, x_vtx), "nil");

        let y_vtx = lenv.get_var("y").expect("y should be registered");
        assert_eq!(get_type_show(&genv, y_vtx), "String");

        let z_vtx = lenv.get_var("z").expect("z should be registered");
        assert_eq!(get_type_show(&genv, z_vtx), "Integer");
    }

    #[test]
    fn test_multi_write_scalar_rhs() {
        // Single non-array expression: first target gets value, rest get nil
        let source = "a, b = 42";
        let (genv, lenv) = analyze(source);

        let a_vtx = lenv.get_var("a").expect("a should be registered");
        assert_eq!(get_type_show(&genv, a_vtx), "Integer");

        let b_vtx = lenv.get_var("b").expect("b should be registered");
        assert_eq!(get_type_show(&genv, b_vtx), "nil");
    }

    #[test]
    fn test_multi_write_rhs_longer_than_lhs() {
        // Extra RHS elements are silently discarded
        let source = "a, b = 1, 2, 3, 4";
        let (genv, lenv) = analyze(source);

        let a_vtx = lenv.get_var("a").expect("a should be registered");
        assert_eq!(get_type_show(&genv, a_vtx), "Integer");

        let b_vtx = lenv.get_var("b").expect("b should be registered");
        assert_eq!(get_type_show(&genv, b_vtx), "Integer");
    }
}
