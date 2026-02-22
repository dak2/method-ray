//! Conditionals - if/unless/case type inference
//!
//! Collects types from each branch and merges them into a Union
//! via edges into a single result Vertex.

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};
use crate::types::Type;
use ruby_prism::{CaseNode, ElseNode, IfNode, Node, UnlessNode, WhenNode};

use super::install::{install_node, install_statements};

/// Process IfNode: if/elsif/else chain
pub(crate) fn process_if_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    if_node: &IfNode,
) -> Option<VertexId> {
    // Process predicate for side effects
    install_node(genv, lenv, changes, source, &if_node.predicate());

    let result_vtx = genv.new_vertex();

    // then branch
    let vtx_then = if_node
        .statements()
        .and_then(|stmts| install_statements(genv, lenv, changes, source, &stmts));
    if let Some(vtx) = vtx_then {
        genv.add_edge(vtx, result_vtx);
    }

    // elsif/else branch (subsequent)
    let has_else = if let Some(subsequent) = if_node.subsequent() {
        let vtx_sub = process_subsequent(genv, lenv, changes, source, &subsequent);
        if let Some(vtx) = vtx_sub {
            genv.add_edge(vtx, result_vtx);
        }
        true
    } else {
        false
    };

    // No else clause → add nil
    if !has_else {
        let nil_vtx = genv.new_source(Type::Nil);
        genv.add_edge(nil_vtx, result_vtx);
    }

    Some(result_vtx)
}

/// Process UnlessNode: unless/else
pub(crate) fn process_unless_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    unless_node: &UnlessNode,
) -> Option<VertexId> {
    // Process predicate for side effects
    install_node(genv, lenv, changes, source, &unless_node.predicate());

    let result_vtx = genv.new_vertex();

    // body branch
    let vtx_body = unless_node
        .statements()
        .and_then(|stmts| install_statements(genv, lenv, changes, source, &stmts));
    if let Some(vtx) = vtx_body {
        genv.add_edge(vtx, result_vtx);
    }

    // else clause
    let has_else = if let Some(else_node) = unless_node.else_clause() {
        let vtx_else = process_else_clause(genv, lenv, changes, source, &else_node);
        if let Some(vtx) = vtx_else {
            genv.add_edge(vtx, result_vtx);
        }
        true
    } else {
        false
    };

    // No else clause → add nil
    if !has_else {
        let nil_vtx = genv.new_source(Type::Nil);
        genv.add_edge(nil_vtx, result_vtx);
    }

    Some(result_vtx)
}

/// Process CaseNode: case/when/else
pub(crate) fn process_case_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    case_node: &CaseNode,
) -> Option<VertexId> {
    // Process predicate for side effects
    if let Some(pred) = case_node.predicate() {
        install_node(genv, lenv, changes, source, &pred);
    }

    let result_vtx = genv.new_vertex();

    // Process each when clause
    for condition in &case_node.conditions() {
        if let Some(when_node) = condition.as_when_node() {
            let vtx_when = process_when_clause(genv, lenv, changes, source, &when_node);
            if let Some(vtx) = vtx_when {
                genv.add_edge(vtx, result_vtx);
            }
        }
    }

    // else clause
    let has_else = if let Some(else_node) = case_node.else_clause() {
        let vtx_else = process_else_clause(genv, lenv, changes, source, &else_node);
        if let Some(vtx) = vtx_else {
            genv.add_edge(vtx, result_vtx);
        }
        true
    } else {
        false
    };

    // No else clause → add nil
    if !has_else {
        let nil_vtx = genv.new_source(Type::Nil);
        genv.add_edge(nil_vtx, result_vtx);
    }

    Some(result_vtx)
}

/// Process subsequent node (elsif chain or else)
fn process_subsequent(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    node: &Node,
) -> Option<VertexId> {
    // elsif: subsequent is another IfNode
    if let Some(if_node) = node.as_if_node() {
        return process_if_node(genv, lenv, changes, source, &if_node);
    }

    // else: subsequent is an ElseNode
    if let Some(else_node) = node.as_else_node() {
        return process_else_clause(genv, lenv, changes, source, &else_node);
    }

    None
}

/// Process ElseNode
fn process_else_clause(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    else_node: &ElseNode,
) -> Option<VertexId> {
    else_node
        .statements()
        .and_then(|stmts| install_statements(genv, lenv, changes, source, &stmts))
}

/// Process WhenNode
fn process_when_clause(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    when_node: &WhenNode,
) -> Option<VertexId> {
    // Process when conditions for side effects
    for cond in &when_node.conditions() {
        install_node(genv, lenv, changes, source, &cond);
    }

    when_node
        .statements()
        .and_then(|stmts| install_statements(genv, lenv, changes, source, &stmts))
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

    // Test 1: if/else basic - different types in branches
    #[test]
    fn test_if_else_basic() {
        let source = r#"
class Foo
  def bar
    if true
      "hello"
    else
      42
    end
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("Foo"), "bar")
            .expect("Foo#bar should be registered");
        let ret_vtx = info.return_vertex.unwrap();
        assert_eq!(get_type_show(&genv, ret_vtx), "(Integer | String)");
    }

    // Test 2: if only (no else) → includes nil
    #[test]
    fn test_if_without_else() {
        let source = r#"
class Foo
  def bar
    if true
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
        assert_eq!(get_type_show(&genv, ret_vtx), "(String | nil)");
    }

    // Test 3: if/elsif/else chain → 3 types
    #[test]
    fn test_if_elsif_else() {
        let source = r#"
class Foo
  def bar
    if true
      "hello"
    elsif false
      42
    else
      true
    end
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
        assert!(type_str.contains("TrueClass"), "should contain TrueClass: {}", type_str);
    }

    // Test 4: unless/else
    #[test]
    fn test_unless_else() {
        let source = r#"
class Foo
  def bar
    unless true
      "a"
    else
      1
    end
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("Foo"), "bar")
            .expect("Foo#bar should be registered");
        let ret_vtx = info.return_vertex.unwrap();
        assert_eq!(get_type_show(&genv, ret_vtx), "(Integer | String)");
    }

    // Test 5: unless without else → includes nil
    #[test]
    fn test_unless_without_else() {
        let source = r#"
class Foo
  def bar
    unless true
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
        assert_eq!(get_type_show(&genv, ret_vtx), "(String | nil)");
    }

    // Test 6: case/when/else
    #[test]
    fn test_case_when_else() {
        let source = r#"
class Foo
  def bar
    case :status
    when :active
      "active"
    when :inactive
      "inactive"
    else
      nil
    end
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("Foo"), "bar")
            .expect("Foo#bar should be registered");
        let ret_vtx = info.return_vertex.unwrap();
        let type_str = get_type_show(&genv, ret_vtx);
        assert!(type_str.contains("String"), "should contain String: {}", type_str);
        assert!(type_str.contains("nil"), "should contain nil: {}", type_str);
    }

    // Test 7: case without else → includes nil
    #[test]
    fn test_case_without_else() {
        let source = r#"
class Foo
  def bar
    case :status
    when :active
      "active"
    when :inactive
      42
    end
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
        assert!(type_str.contains("nil"), "should contain nil: {}", type_str);
    }

    // Test 8: conditional inside method → return type reflects union
    #[test]
    fn test_conditional_in_method_return() {
        let source = r#"
class Converter
  def convert(x)
    if true
      "text"
    else
      100
    end
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("Converter"), "convert")
            .expect("Converter#convert should be registered");
        let ret_vtx = info.return_vertex.unwrap();
        assert_eq!(get_type_show(&genv, ret_vtx), "(Integer | String)");
    }

    // Test 9: nested conditionals
    #[test]
    fn test_nested_conditionals() {
        let source = r#"
class Foo
  def bar
    if true
      if false
        "inner"
      else
        42
      end
    else
      :sym
    end
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("Foo"), "bar")
            .expect("Foo#bar should be registered");
        let ret_vtx = info.return_vertex.unwrap();
        let type_str = get_type_show(&genv, ret_vtx);
        // Outer if: inner_if_result | Symbol
        // Inner if: (Integer | String) → propagates through
        assert!(type_str.contains("Symbol"), "should contain Symbol: {}", type_str);
    }

    // Test 10: all branches same type → single type (not union)
    #[test]
    fn test_same_type_branches() {
        let source = r#"
class Foo
  def bar
    if true
      "hello"
    else
      "world"
    end
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

    // Test 11: ternary operator - different types → union
    #[test]
    fn test_ternary_union_type() {
        let source = r#"
class Foo
  def bar
    true ? "hello" : 42
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("Foo"), "bar")
            .expect("Foo#bar should be registered");
        let ret_vtx = info.return_vertex.unwrap();
        assert_eq!(get_type_show(&genv, ret_vtx), "(Integer | String)");
    }

    // Test 12: ternary operator - same type → single type
    #[test]
    fn test_ternary_same_type() {
        let source = r#"
class Foo
  def bar
    true ? "hello" : "world"
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

    // Test 13: ternary operator - nil branch → union with nil
    #[test]
    fn test_ternary_nil_branch() {
        let source = r#"
class Foo
  def bar
    true ? "hello" : nil
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("Foo"), "bar")
            .expect("Foo#bar should be registered");
        let ret_vtx = info.return_vertex.unwrap();
        assert_eq!(get_type_show(&genv, ret_vtx), "(String | nil)");
    }

    // Test 14: nested ternary operator
    #[test]
    fn test_ternary_nested() {
        let source = r#"
class Foo
  def bar
    true ? (false ? "inner" : 42) : :sym
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("Foo"), "bar")
            .expect("Foo#bar should be registered");
        let ret_vtx = info.return_vertex.unwrap();
        assert_eq!(get_type_show(&genv, ret_vtx), "(Integer | String | Symbol)");
    }
}
