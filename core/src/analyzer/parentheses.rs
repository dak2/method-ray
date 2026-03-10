//! Parentheses - pass-through type propagation for parenthesized expressions

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};

use super::install::{install_node, install_statements};

/// Process ParenthesesNode: propagate inner expression's type
pub(crate) fn process_parentheses_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    paren_node: &ruby_prism::ParenthesesNode,
) -> Option<VertexId> {
    let body = paren_node.body()?;

    if let Some(stmts) = body.as_statements_node() {
        // (expr1; expr2) → process all, return last expression's type
        install_statements(genv, lenv, changes, source, &stmts)
    } else {
        // (expr) → propagate inner expression's type directly
        install_node(genv, lenv, changes, source, &body)
    }
}

#[cfg(test)]
mod tests {
    use crate::analyzer::install::AstInstaller;
    use crate::env::{GlobalEnv, LocalEnv};
    use crate::graph::VertexId;
    use crate::parser::ParseSession;
    use crate::types::Type;

    fn analyze(source: &str) -> GlobalEnv {
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

        genv
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
    fn test_parenthesized_integer() {
        let source = r#"
class Foo
  def bar
    x = (42)
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("Foo"), "bar")
            .expect("Foo#bar should be registered");
        let ret_vtx = info.return_vertex.unwrap();
        assert_eq!(get_type_show(&genv, ret_vtx), "Integer");
    }

    #[test]
    fn test_parenthesized_string() {
        let source = r#"
class Foo
  def bar
    x = ("hello")
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("Foo"), "bar")
            .expect("Foo#bar should be registered");
        let ret_vtx = info.return_vertex.unwrap();
        assert_eq!(get_type_show(&genv, ret_vtx), "String");
    }

    #[test]
    fn test_parenthesized_multiple_statements() {
        let source = r#"
class Foo
  def bar
    x = (a = 1; "hello")
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("Foo"), "bar")
            .expect("Foo#bar should be registered");
        let ret_vtx = info.return_vertex.unwrap();
        assert_eq!(get_type_show(&genv, ret_vtx), "String");
    }
}
