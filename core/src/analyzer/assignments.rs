//! Multiple Assignment Handlers - Processing Ruby multiple assignment
//!
//! v0.1.8 scope: Only RHS as ArrayNode (multiple literal values) is supported.
//! TODO: Support RHS as single expression (array decomposition)
//! TODO: Support splat target (*rest) as Array type
//! TODO: Support RHS as method return value decomposition
//! TODO: When LHS is longer than RHS, register trailing targets as NilClass

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};

use super::bytes_to_name;
use super::variables::install_local_var_write;

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
        for (target, rhs_elem) in node.lefts().iter().zip(array_node.elements().iter()) {
            if let Some(target_node) = target.as_local_variable_target_node() {
                let var_name = bytes_to_name(target_node.name().as_slice());
                let rhs_vtx =
                    super::install::install_node(genv, lenv, changes, source, &rhs_elem);
                if let Some(rv) = rhs_vtx {
                    last_vtx = Some(install_local_var_write(genv, lenv, changes, var_name, rv));
                } else {
                    let var_vtx = genv.new_vertex();
                    lenv.new_var(var_name, var_vtx);
                    last_vtx = Some(var_vtx);
                }
            }
        }
    } else {
        for target in node.lefts().iter() {
            if let Some(target_node) = target.as_local_variable_target_node() {
                let var_name = bytes_to_name(target_node.name().as_slice());
                let var_vtx = genv.new_vertex();
                lenv.new_var(var_name, var_vtx);
                last_vtx = Some(var_vtx);
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
        let (_, lenv) = analyze(source);

        assert!(lenv.get_var("a").is_some(), "a should be registered");
        assert!(lenv.get_var("b").is_some(), "b should be registered");
        // KNOWN LIMITATION (v0.1.8): In Ruby, c receives nil, but zip skips it here
        assert!(
            lenv.get_var("c").is_none(),
            "c should not be registered (zip skips)"
        );
    }

    #[test]
    fn test_multi_write_does_not_panic_on_non_array_rhs() {
        let source = "a, b = some_expr";
        let (_, lenv) = analyze(source);

        // Variables should be registered (untyped) without panic
        assert!(lenv.get_var("a").is_some(), "a should be registered");
        assert!(lenv.get_var("b").is_some(), "b should be registered");
    }
}
