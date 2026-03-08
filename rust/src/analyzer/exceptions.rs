//! Exceptions - begin/rescue/ensure type inference
//!
//! Collects types from each branch and merges them into a Union
//! via edges into a single result Vertex.
//! Applies the same MergeVertex pattern as conditionals.rs.

use crate::env::{GlobalEnv, LocalEnv};
use crate::graph::{ChangeSet, VertexId};
use crate::types::Type;
use ruby_prism::{BeginNode, RescueModifierNode, RescueNode};

use super::bytes_to_name;
use super::install::{install_node, install_statements};

/// Process BeginNode: begin/rescue/else/ensure
///
/// Type aggregation rules:
///   - No rescue clause: return begin body type directly
///   - With else clause: else type + all rescue types → Union (begin body excluded)
///   - Without else clause: begin body type + all rescue types → Union
///   - Ensure clause: processed for side effects only, does not affect return type
pub(crate) fn process_begin_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    begin_node: &BeginNode,
) -> Option<VertexId> {
    let begin_vtx = begin_node
        .statements()
        .and_then(|s| install_statements(genv, lenv, changes, source, &s));

    let result = if let Some(rescue_node) = begin_node.rescue_clause() {
        let result_vtx = genv.new_vertex();

        process_rescue_chain(genv, lenv, changes, source, &rescue_node, result_vtx);

        if let Some(else_node) = begin_node.else_clause() {
            // With else: else type replaces begin body type (Ruby spec)
            let else_vtx = else_node
                .statements()
                .and_then(|s| install_statements(genv, lenv, changes, source, &s));
            if let Some(vtx) = else_vtx {
                genv.add_edge(vtx, result_vtx);
            }
        } else if let Some(vtx) = begin_vtx {
            genv.add_edge(vtx, result_vtx);
        }

        Some(result_vtx)
    } else {
        begin_vtx
    };

    // Ensure: side effects only, does not affect return type
    if let Some(ensure_node) = begin_node.ensure_clause() {
        if let Some(stmts) = ensure_node.statements() {
            let _ = install_statements(genv, lenv, changes, source, &stmts);
        }
    }

    result
}

/// Process RescueNode chain recursively.
/// Empty rescue body evaluates to nil.
fn process_rescue_chain(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    rescue_node: &RescueNode,
    result_vtx: VertexId,
) {
    let body_vtx = process_rescue_body(genv, lenv, changes, source, rescue_node);
    let vtx = body_vtx.unwrap_or_else(|| genv.new_source(Type::Nil));
    genv.add_edge(vtx, result_vtx);

    if let Some(next) = rescue_node.subsequent() {
        process_rescue_chain(genv, lenv, changes, source, &next, result_vtx);
    }
}

/// Process a single RescueNode body.
/// Registers the rescue variable (=> e), processes the body,
/// then removes the variable from scope.
fn process_rescue_body(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    rescue_node: &RescueNode,
) -> Option<VertexId> {
    for exc in &rescue_node.exceptions() {
        install_node(genv, lenv, changes, source, &exc);
    }

    // Save/restore rescue variable binding (=> e)
    // TODO: Only LocalVariableTargetNode is handled; instance/global/class vars are not yet supported.
    // TODO: Always typed as StandardError regardless of declared exception class.
    let var_binding = if let Some(ref_node) = rescue_node.reference() {
        ref_node.as_local_variable_target_node().map(|target| {
            let name = bytes_to_name(target.name().as_slice());
            let saved = lenv.get_var(&name);
            let exception_vtx = genv.new_vertex();
            let std_err_src = genv.new_source(Type::instance("StandardError"));
            genv.add_edge(std_err_src, exception_vtx);
            lenv.new_var(name.clone(), exception_vtx);
            (name, saved)
        })
    } else {
        None
    };

    let body_vtx = rescue_node
        .statements()
        .and_then(|s| install_statements(genv, lenv, changes, source, &s));

    if let Some((name, saved)) = var_binding {
        match saved {
            Some(prev_vtx) => lenv.new_var(name, prev_vtx),
            None => lenv.remove_var(&name),
        }
    }

    body_vtx
}

/// Process RescueModifierNode: `expression rescue rescue_expression`
pub(crate) fn process_rescue_modifier_node(
    genv: &mut GlobalEnv,
    lenv: &mut LocalEnv,
    changes: &mut ChangeSet,
    source: &str,
    node: &RescueModifierNode,
) -> Option<VertexId> {
    let result_vtx = genv.new_vertex();

    let expr_vtx = install_node(genv, lenv, changes, source, &node.expression());
    if let Some(vtx) = expr_vtx {
        genv.add_edge(vtx, result_vtx);
    }

    let rescue_vtx = install_node(genv, lenv, changes, source, &node.rescue_expression());
    if let Some(vtx) = rescue_vtx {
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
    fn test_begin_rescue_basic() {
        let source = r#"
class Foo
  def bar
    begin
      "hello"
    rescue
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

    #[test]
    fn test_begin_rescue_else() {
        let source = r#"
class Foo
  def bar
    begin
      "hello"
    rescue
      42
    else
      :ok
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
        // else present: begin body excluded, else + rescue types
        assert!(type_str.contains("Symbol"), "should contain Symbol: {}", type_str);
        assert!(type_str.contains("Integer"), "should contain Integer: {}", type_str);
        assert!(!type_str.contains("String"), "should NOT contain String: {}", type_str);
    }

    #[test]
    fn test_begin_ensure_only() {
        let source = r#"
class Foo
  def bar
    begin
      "hello"
    ensure
      puts "cleanup"
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

    #[test]
    fn test_begin_rescue_ensure() {
        let source = r#"
class Foo
  def bar
    begin
      "hello"
    rescue
      42
    ensure
      :cleanup
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
        // ensure does not affect return type
        assert_eq!(type_str, "(Integer | String)");
    }

    #[test]
    fn test_rescue_variable_type() {
        let source = r#"
class Foo
  def bar
    begin
      "hello"
    rescue => e
      e
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
        assert!(
            type_str.contains("StandardError"),
            "should contain StandardError: {}",
            type_str
        );
    }

    #[test]
    fn test_multiple_rescue_clauses() {
        let source = r#"
class Foo
  def bar
    begin
      "hello"
    rescue ArgumentError
      42
    rescue RuntimeError
      :error
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
        assert!(type_str.contains("Integer"), "should contain Integer: {}", type_str);
        assert!(type_str.contains("Symbol"), "should contain Symbol: {}", type_str);
    }

    #[test]
    fn test_rescue_modifier_basic() {
        let source = r#"
class Foo
  def bar
    "hello" rescue 42
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

    #[test]
    fn test_rescue_modifier_same_type() {
        let source = r#"
class Foo
  def bar
    "hello" rescue "world"
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
    fn test_nested_begin_rescue() {
        let source = r#"
class Foo
  def bar
    begin
      begin
        "inner"
      rescue
        42
      end
    rescue
      :outer
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
        // Outer: Union(inner_begin_rescue | :outer) = (Integer | String | Symbol)
        assert!(type_str.contains("Integer"), "should contain Integer: {}", type_str);
        assert!(type_str.contains("String"), "should contain String: {}", type_str);
        assert!(type_str.contains("Symbol"), "should contain Symbol: {}", type_str);
    }

    #[test]
    fn test_begin_rescue_in_method() {
        let source = r#"
class Foo
  def bar
    x = begin
      "hello"
    rescue
      42
    end
    x
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

    #[test]
    fn test_ensure_side_effects() {
        // ensure body should be processed (no panic) but not affect return type
        let source = r#"
class Foo
  def bar
    begin
      "hello"
    rescue
      42
    ensure
      x = "side_effect"
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

    #[test]
    fn test_rescue_variable_scope_restore() {
        // Rescue variable should not destroy outer binding
        let source = r#"
class Foo
  def bar
    e = "outer"
    begin
      "hello"
    rescue => e
      e
    end
    e
  end
end
"#;
        let genv = analyze(source);
        let info = genv
            .resolve_method(&Type::instance("Foo"), "bar")
            .expect("Foo#bar should be registered");
        let ret_vtx = info.return_vertex.unwrap();
        let type_str = get_type_show(&genv, ret_vtx);
        // After rescue block, e should be restored to outer binding (String)
        assert!(type_str.contains("String"), "should contain String: {}", type_str);
    }

    #[test]
    fn test_rescue_variable_scope_removal() {
        // When rescue variable has no prior binding, it should be removed after rescue block
        let source = r#"
class Foo
  def bar
    begin
      "hello"
    rescue => e
      e
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
        // begin body (String) + rescue body where e = StandardError
        assert!(type_str.contains("String"), "should contain String: {}", type_str);
        assert!(
            type_str.contains("StandardError"),
            "should contain StandardError: {}",
            type_str
        );
    }

    #[test]
    fn test_empty_rescue_body_is_nil() {
        let source = r#"
class Foo
  def bar
    begin
      "hello"
    rescue
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
}
