//! Operators - logical operator type inference (&&, ||, !)

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};
use crate::types::Type;
use ruby_prism::{AndNode, Node, OrNode};

use super::install::install_node;

/// Merge two branch nodes into a union type vertex.
fn process_binary_logical_op<'a>(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    left: Node<'a>,
    right: Node<'a>,
) -> Option<VertexId> {
    let result_vtx = genv.new_vertex();

    if let Some(vtx) = install_node(genv, lenv, changes, source, &left) {
        genv.add_edge(vtx, result_vtx);
    }

    if let Some(vtx) = install_node(genv, lenv, changes, source, &right) {
        genv.add_edge(vtx, result_vtx);
    }

    Some(result_vtx)
}

/// Process AndNode (a && b): Union(type(a), type(b))
///
/// Runtime: if `a` is falsy, returns `a`; otherwise returns `b`.
/// Static: conservatively produce Union(type(a), type(b)).
pub(crate) fn process_and_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    and_node: &AndNode,
) -> Option<VertexId> {
    process_binary_logical_op(genv, lenv, changes, source, and_node.left(), and_node.right())
}

/// Process OrNode (a || b): Union(type(a), type(b))
///
/// Runtime: if `a` is truthy, returns `a`; otherwise returns `b`.
/// Static: conservatively produce Union(type(a), type(b)).
pub(crate) fn process_or_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    or_node: &OrNode,
) -> Option<VertexId> {
    process_binary_logical_op(genv, lenv, changes, source, or_node.left(), or_node.right())
}

/// Process not operator (!expr): TrueClass | FalseClass
///
/// In ruby-prism, `!expr` is represented as a CallNode with method name "!".
/// Static approximation: we cannot determine the receiver's truthiness at
/// compile time, so conservatively return TrueClass | FalseClass for any `!` call.
/// In practice, `!nil` and `!false` are always true, but we do not track that here.
///
/// Receiver side effects are already analyzed by the caller (process_needs_child).
///
/// TODO: Ruby allows overriding `BasicObject#!`. Currently we always return
/// TrueClass | FalseClass, ignoring user-defined `!` methods. If needed, look up
/// the receiver's RBS definition and use its return type instead.
pub(crate) fn process_not_operator(genv: &mut GlobalEnv) -> VertexId {
    let result_vtx = genv.new_vertex();
    let true_vtx = genv.new_source(Type::instance("TrueClass"));
    let false_vtx = genv.new_source(Type::instance("FalseClass"));
    genv.add_edge(true_vtx, result_vtx);
    genv.add_edge(false_vtx, result_vtx);
    result_vtx
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

    // ============================================
    // Not operator (!) tests
    // ============================================

    #[test]
    fn test_not_operator_returns_boolean_union() {
        let source = r#"
class Foo
  def bar
    !true
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("Foo"), "bar")
            .expect("bar should be registered");
        let ty = get_type_show(&genv, info.return_vertex.unwrap());
        assert!(ty.contains("TrueClass"), "expected TrueClass in {}", ty);
        assert!(ty.contains("FalseClass"), "expected FalseClass in {}", ty);
    }

    #[test]
    fn test_not_operator_receiver_side_effects_analyzed() {
        let source = r#"
class Foo
  def bar
    !(1.upcase)
  end
end
"#;
        let genv = analyze(source);
        assert!(
            !genv.type_errors.is_empty(),
            "expected type error for Integer#upcase"
        );
    }

    #[test]
    fn test_double_not_operator_union() {
        let source = r#"
class Foo
  def bar
    !!true
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("Foo"), "bar")
            .expect("bar should be registered");
        let ty = get_type_show(&genv, info.return_vertex.unwrap());
        assert!(ty.contains("TrueClass"), "expected TrueClass in {}", ty);
        assert!(ty.contains("FalseClass"), "expected FalseClass in {}", ty);
    }

    #[test]
    fn test_not_nil_returns_boolean() {
        let source = r#"
class Foo
  def bar
    !nil
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("Foo"), "bar")
            .expect("bar should be registered");
        let ty = get_type_show(&genv, info.return_vertex.unwrap());
        assert!(ty.contains("TrueClass"), "expected TrueClass in {}", ty);
        assert!(ty.contains("FalseClass"), "expected FalseClass in {}", ty);
    }
}
