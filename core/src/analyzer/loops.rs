//! Loops - while/until loop type inference
//!
//! Ruby loop expressions evaluate to nil (except when break passes a value).
//! Break value propagation is not yet supported.

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};
use crate::types::Type;
use ruby_prism::{ForNode, UntilNode, WhileNode};

use super::bytes_to_name;
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

/// Process ForNode: `for index in collection; statements; end`
///
/// Ruby's `for` does NOT create a new scope — the loop variable persists
/// after the loop. This differs from `collection.each { |x| }` which
/// creates a block scope.
///
/// Returns nil (consistent with while/until; Ruby's for returns the
/// collection, but the return value is rarely used in practice).
pub(crate) fn process_for_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    for_node: &ForNode,
) -> Option<VertexId> {
    let collection_vtx = install_node(genv, lenv, changes, source, &for_node.collection());

    // TODO: MultiTargetNode (e.g., `for a, b in [[1, "x"]]`) is not yet supported
    if let Some(target) = for_node.index().as_local_variable_target_node() {
        let name = bytes_to_name(target.name().as_slice());
        let var_vtx = genv.new_vertex();

        // Array[T] or Range[T] → loop var gets T
        let elem_type = collection_vtx
            .and_then(|vtx| genv.get_source(vtx))
            .and_then(|src| src.ty.type_args())
            .and_then(|args| args.first().cloned());
        if let Some(ty) = elem_type {
            let elem_src = genv.new_source(ty);
            genv.add_edge(elem_src, var_vtx);
        }

        lenv.new_var(name, var_vtx);
    }

    if let Some(stmts) = for_node.statements() {
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

    // --- for loop tests ---

    #[test]
    fn test_for_returns_nil() {
        let source = r#"
class Foo
  def bar
    for x in [1, 2, 3]
      x
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
    fn test_for_variable_type_from_array() {
        let source = r#"
for item in [1, 2, 3]
  item
end
"#;
        // Should not panic; loop variable is registered
        analyze(source);
    }

    #[test]
    fn test_for_variable_persists_after_loop() {
        // for does NOT create a new scope — variable persists
        let source = r#"
class Foo
  def bar
    for x in [1, 2, 3]
    end
    x
  end
end
"#;
        // Should not panic — x is accessible after the loop
        analyze(source);
    }

    #[test]
    fn test_for_empty_body() {
        let source = r#"
class Foo
  def bar
    for x in [1, 2, 3]
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
    fn test_for_with_method_call_in_body() {
        // Should not panic — method call on loop variable is processed
        // (type error check requires RBS, covered by Ruby integration test)
        let source = r#"
class Foo
  def bar
    for item in ["hello", "world"]
      item.upcase
    end
  end
end
"#;
        analyze(source);
    }
}
