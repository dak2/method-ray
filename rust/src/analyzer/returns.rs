//! Return statement handling
//!
//! Processes `return expr` by connecting the expression's vertex
//! to the enclosing method's merge vertex.

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};

use super::install::install_node;

/// Process ReturnNode: connect return value to method's merge vertex
pub(crate) fn process_return_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    return_node: &ruby_prism::ReturnNode,
) -> Option<VertexId> {
    // Process return value (first argument only; multi-value return not yet supported)
    let value_vtx = if let Some(arguments) = return_node.arguments() {
        arguments
            .arguments()
            .iter()
            .next()
            .and_then(|arg| install_node(genv, lenv, changes, source, &arg))
    } else {
        // `return` without value → nil
        Some(genv.new_source(crate::types::Type::Nil))
    };

    // Connect return value to method's merge vertex
    if let Some(vtx) = value_vtx {
        if let Some(merge_vtx) = genv.scope_manager.current_method_return_vertex() {
            genv.add_edge(vtx, merge_vtx);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use crate::env::{GlobalEnv, LocalEnv};
    use crate::graph::ChangeSet;
    use crate::parser::ParseSession;
    use crate::types::Type;

    fn setup_and_infer(source: &str) -> GlobalEnv {
        let session = ParseSession::new();
        let parse_result = session.parse_source(source, "test.rb").unwrap();
        let root = parse_result.node();
        let program = root.as_program_node().unwrap();

        let mut genv = GlobalEnv::new();
        let mut lenv = LocalEnv::new();
        let mut changes = ChangeSet::new();

        for stmt in &program.statements().body() {
            crate::analyzer::install::install_node(
                &mut genv, &mut lenv, &mut changes, source, &stmt,
            );
        }

        genv.apply_changes(changes);
        genv.run_all();
        genv
    }

    fn get_return_type(genv: &GlobalEnv, class_name: &str, method_name: &str) -> String {
        let info = genv
            .resolve_method(&Type::instance(class_name), method_name)
            .unwrap_or_else(|| panic!("{}#{} should be registered", class_name, method_name));
        let vtx = info
            .return_vertex
            .expect("return_vertex should be Some");

        if let Some(source) = genv.get_source(vtx) {
            source.ty.show()
        } else if let Some(vertex) = genv.get_vertex(vtx) {
            vertex.show()
        } else {
            panic!("return_vertex not found");
        }
    }

    #[test]
    fn test_simple_return() {
        let source = r#"
class Foo
  def bar
    return "hello"
  end
end
"#;
        let genv = setup_and_infer(source);
        assert_eq!(get_return_type(&genv, "Foo", "bar"), "String");
    }

    #[test]
    fn test_return_with_implicit_return_union() {
        let source = r#"
class Foo
  def bar
    return "hello" if true
    42
  end
end
"#;
        let genv = setup_and_infer(source);
        let ty = get_return_type(&genv, "Foo", "bar");
        assert!(ty.contains("Integer"), "should contain Integer, got: {}", ty);
        assert!(ty.contains("String"), "should contain String, got: {}", ty);
    }

    #[test]
    fn test_multiple_returns() {
        let source = r#"
class Foo
  def bar
    return "a" if true
    return :b if false
    42
  end
end
"#;
        let genv = setup_and_infer(source);
        let ty = get_return_type(&genv, "Foo", "bar");
        assert!(ty.contains("Integer"), "should contain Integer, got: {}", ty);
        assert!(ty.contains("String"), "should contain String, got: {}", ty);
        assert!(ty.contains("Symbol"), "should contain Symbol, got: {}", ty);
    }

    #[test]
    fn test_return_without_value() {
        let source = r#"
class Foo
  def bar
    return if true
    42
  end
end
"#;
        let genv = setup_and_infer(source);
        let ty = get_return_type(&genv, "Foo", "bar");
        assert!(ty.contains("Integer"), "should contain Integer, got: {}", ty);
        assert!(ty.contains("nil"), "should contain nil, got: {}", ty);
    }

    #[test]
    fn test_no_return_backward_compat() {
        let source = r#"
class Foo
  def bar
    "hello"
  end
end
"#;
        let genv = setup_and_infer(source);
        assert_eq!(get_return_type(&genv, "Foo", "bar"), "String");
    }

    #[test]
    fn test_return_only_method() {
        let source = r#"
class Foo
  def bar
    return "hello"
  end
end
"#;
        let genv = setup_and_infer(source);
        assert_eq!(get_return_type(&genv, "Foo", "bar"), "String");
    }

    #[test]
    fn test_return_dead_code_over_approximation() {
        let source = r#"
class Foo
  def bar
    return "hello"
    42
  end
end
"#;
        let genv = setup_and_infer(source);
        let ty = get_return_type(&genv, "Foo", "bar");
        // Dead code after return is still processed (over-approximation)
        assert!(ty.contains("Integer"), "should contain Integer (dead code), got: {}", ty);
        assert!(ty.contains("String"), "should contain String, got: {}", ty);
    }
}
