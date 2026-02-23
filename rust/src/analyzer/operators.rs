//! Operators - logical operator type inference (&&, ||)

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};
use ruby_prism::{AndNode, OrNode};

use super::install::install_node;

/// Process AndNode (a && b): Union(type(a), type(b))
///
/// Short-circuit semantics: if `a` is falsy, returns `a`; otherwise returns `b`.
/// Static approximation: we cannot determine truthiness at compile time,
/// so we conservatively produce Union(type(a), type(b)).
pub(crate) fn process_and_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    and_node: &AndNode,
) -> Option<VertexId> {
    let result_vtx = genv.new_vertex();

    let left_vtx = install_node(genv, lenv, changes, source, &and_node.left());
    if let Some(vtx) = left_vtx {
        genv.add_edge(vtx, result_vtx);
    }

    let right_vtx = install_node(genv, lenv, changes, source, &and_node.right());
    if let Some(vtx) = right_vtx {
        genv.add_edge(vtx, result_vtx);
    }

    Some(result_vtx)
}

/// Process OrNode (a || b): Union(type(a), type(b))
///
/// Short-circuit semantics: if `a` is truthy, returns `a`; otherwise returns `b`.
/// Static approximation: identical to AndNode — Union of both sides.
pub(crate) fn process_or_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    or_node: &OrNode,
) -> Option<VertexId> {
    let result_vtx = genv.new_vertex();

    let left_vtx = install_node(genv, lenv, changes, source, &or_node.left());
    if let Some(vtx) = left_vtx {
        genv.add_edge(vtx, result_vtx);
    }

    let right_vtx = install_node(genv, lenv, changes, source, &or_node.right());
    if let Some(vtx) = right_vtx {
        genv.add_edge(vtx, result_vtx);
    }

    Some(result_vtx)
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

    /// Helper: get the type string for a vertex ID
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
    fn test_and_node_union_type() {
        let source = r#"
class Foo
  def bar
    true && "hello"
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("Foo"), "bar")
            .expect("Foo#bar should be registered");
        let ret_vtx = info.return_vertex.unwrap();
        let type_str = get_type_show(&genv, ret_vtx);
        assert!(type_str.contains("TrueClass"), "should contain TrueClass: {}", type_str);
        assert!(type_str.contains("String"), "should contain String: {}", type_str);
    }

    #[test]
    fn test_and_node_same_type() {
        let source = r#"
class Foo
  def bar
    "a" && "b"
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
    fn test_or_node_union_type() {
        let source = r#"
class Foo
  def bar
    42 || "hello"
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("Foo"), "bar")
            .expect("Foo#bar should be registered");
        let ret_vtx = info.return_vertex.unwrap();
        let type_str = get_type_show(&genv, ret_vtx);
        assert!(type_str.contains("Integer"), "should contain Integer: {}", type_str);
        assert!(type_str.contains("String"), "should contain String: {}", type_str);
    }

    #[test]
    fn test_or_node_same_type() {
        let source = r#"
class Foo
  def bar
    1 || 2
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
    fn test_nested_logical_operators() {
        let source = r#"
class Foo
  def bar
    1 && "a" || :b
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("Foo"), "bar")
            .expect("Foo#bar should be registered");
        let ret_vtx = info.return_vertex.unwrap();
        let type_str = get_type_show(&genv, ret_vtx);
        assert!(type_str.contains("Integer"), "should contain Integer: {}", type_str);
        assert!(type_str.contains("String"), "should contain String: {}", type_str);
        assert!(type_str.contains("Symbol"), "should contain Symbol: {}", type_str);
    }

}
