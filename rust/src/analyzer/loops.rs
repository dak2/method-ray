//! Loops - while/until loop type inference
//!
//! Ruby loop expressions evaluate to nil (except when break passes a value).
//! Break value propagation is not yet supported.

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};
use crate::types::Type;
use ruby_prism::{UntilNode, WhileNode};

use super::install::{install_node, install_statements};

/// Process WhileNode: `while predicate; statements; end`
///
/// Returns nil. Traverses predicate and body to register method calls
/// and variable assignments in the type graph.
pub(crate) fn process_while_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    while_node: &WhileNode,
) -> Option<VertexId> {
    install_node(genv, lenv, changes, source, &while_node.predicate());

    if let Some(stmts) = while_node.statements() {
        install_statements(genv, lenv, changes, source, &stmts);
    }

    Some(genv.new_source(Type::Nil))
}

/// Process UntilNode: `until predicate; statements; end`
///
/// Returns nil. Traverses predicate and body to register method calls
/// and variable assignments in the type graph.
pub(crate) fn process_until_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    until_node: &UntilNode,
) -> Option<VertexId> {
    install_node(genv, lenv, changes, source, &until_node.predicate());

    if let Some(stmts) = until_node.statements() {
        install_statements(genv, lenv, changes, source, &stmts);
    }

    Some(genv.new_source(Type::Nil))
}

#[cfg(test)]
mod tests {
    use crate::analyzer::install::AstInstaller;
    use crate::env::{GlobalEnv, LocalEnv};
    use crate::graph::VertexId;
    use crate::parser::ParseSession;
    use crate::types::Type;

    /// Helper: parse Ruby source, process with AstInstaller, and return GlobalEnv
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

    /// Helper: get the type string for a vertex ID (checks both Vertex and Source)
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
    fn test_while_returns_nil() {
        let source = r#"
class Foo
  def bar
    while true
      "hello"
    end
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("Foo"), "bar")
            .expect("Foo#bar should be registered");
        let ret_vtx = info.return_vertex.unwrap();
        assert_eq!(get_type_show(&genv, ret_vtx), "nil");
    }

    #[test]
    fn test_until_returns_nil() {
        let source = r#"
class Foo
  def bar
    until false
      "hello"
    end
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("Foo"), "bar")
            .expect("Foo#bar should be registered");
        let ret_vtx = info.return_vertex.unwrap();
        assert_eq!(get_type_show(&genv, ret_vtx), "nil");
    }

    #[test]
    fn test_while_variable_assignment_in_body() {
        // Should not panic — variable assignment inside loop is processed
        let source = r#"
x = "initial"
while true
  x = "hello"
end
"#;
        analyze(source);
    }

    #[test]
    fn test_while_modifier_form() {
        let source = r#"
class Foo
  def bar
    x = "hello" while false
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("Foo"), "bar")
            .expect("Foo#bar should be registered");
        let ret_vtx = info.return_vertex.unwrap();
        assert_eq!(get_type_show(&genv, ret_vtx), "nil");
    }

    #[test]
    fn test_begin_end_while() {
        let source = r#"
class Foo
  def bar
    begin
      "hello"
    end while false
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("Foo"), "bar")
            .expect("Foo#bar should be registered");
        let ret_vtx = info.return_vertex.unwrap();
        assert_eq!(get_type_show(&genv, ret_vtx), "nil");
    }
}
