//! Integration Tests - End-to-end analyzer tests
//!
//! This module contains integration tests that verify:
//! - Class/method definition handling
//! - Instance variable type tracking across methods
//! - Type error detection for undefined methods
//! - Method chain type inference

use crate::analyzer::AstInstaller;
use crate::env::{GlobalEnv, LocalEnv};
use crate::parser::parse_ruby_source;
use crate::types::Type;

/// Helper to run analysis on Ruby source code
fn analyze(source: &str) -> (GlobalEnv, LocalEnv) {
    let parse_result = parse_ruby_source(source, "test.rb".to_string()).unwrap();

    let mut genv = GlobalEnv::new();

    // Register common methods
    genv.register_builtin_method(Type::string(), "upcase", Type::string());
    genv.register_builtin_method(Type::string(), "downcase", Type::string());

    let mut lenv = LocalEnv::new();
    let mut installer = AstInstaller::new(&mut genv, &mut lenv, source);

    let root = parse_result.node();

    if let Some(program_node) = root.as_program_node() {
        let statements = program_node.statements();
        for stmt in &statements.body() {
            installer.install_node(&stmt);
        }
    }

    installer.finish();

    (genv, lenv)
}

#[test]
fn test_class_method_error_detection() {
    let source = r#"
class User
  def test
    x = 123
    y = x.upcase
  end
end
"#;

    let (genv, _lenv) = analyze(source);

    // Type error should be detected: Integer doesn't have upcase method
    assert_eq!(genv.type_errors.len(), 1);
    assert_eq!(genv.type_errors[0].method_name, "upcase");
}

#[test]
fn test_class_with_instance_variable() {
    let source = r#"
class User
  def initialize
    @name = "John"
  end

  def greet
    @name.upcase
  end
end
"#;

    let (genv, _lenv) = analyze(source);

    // No type errors should occur - @name is String
    assert_eq!(genv.type_errors.len(), 0);
}

#[test]
fn test_instance_variable_type_error() {
    let source = r#"
class User
  def initialize
    @name = 123
  end

  def greet
    @name.upcase
  end
end
"#;

    let (genv, _lenv) = analyze(source);

    // Type error should be detected: @name is Integer, not String
    assert_eq!(genv.type_errors.len(), 1);
    assert_eq!(genv.type_errors[0].method_name, "upcase");
}

#[test]
fn test_multiple_classes() {
    let source = r#"
class User
  def name
    x = 123
    x.upcase
  end
end

class Post
  def title
    y = "hello"
    y.upcase
  end
end
"#;

    let (genv, _lenv) = analyze(source);

    // Only User#name should have error (Integer#upcase), Post#title is fine
    assert_eq!(genv.type_errors.len(), 1);
    assert_eq!(genv.type_errors[0].method_name, "upcase");
}

#[test]
fn test_method_chain() {
    let source = r#"
x = "hello"
y = x.upcase.downcase
"#;

    let (genv, lenv) = analyze(source);

    let y_vtx = lenv.get_var("y").unwrap();
    assert_eq!(genv.get_vertex(y_vtx).unwrap().show(), "String");
}

// ============================================
// Method Parameter Tests
// ============================================

#[test]
fn test_method_parameter_available_as_local_var() {
    let source = r#"
def greet(name)
  x = name
end
"#;

    let (genv, _lenv) = analyze(source);

    // No type errors should occur - name parameter should be available
    assert_eq!(genv.type_errors.len(), 0);
}

#[test]
fn test_method_multiple_parameters() {
    let source = r#"
def calculate(a, b, c)
  x = a
  y = b
  z = c
end
"#;

    let (genv, _lenv) = analyze(source);

    // No type errors should occur - all parameters should be available
    assert_eq!(genv.type_errors.len(), 0);
}

#[test]
fn test_class_method_with_parameter() {
    let source = r#"
class User
  def initialize(name)
    @name = name
  end
end
"#;

    let (genv, _lenv) = analyze(source);

    // No type errors should occur
    assert_eq!(genv.type_errors.len(), 0);
}

#[test]
fn test_parameter_method_call() {
    // Parameter has Bot (untyped) type, so method calls won't error
    // because we can't verify if the method exists on an untyped value
    let source = r#"
def greet(name)
  name.upcase
end
"#;

    let (genv, _lenv) = analyze(source);

    // With Bot type, we don't know if upcase exists or not
    // Current behavior: Bot type means no method resolution, so no error
    // This is acceptable for Phase 3 - we can improve later with call-site inference
    assert!(
        genv.type_errors.is_empty(),
        "Bot (untyped) parameters should not produce method errors"
    );
}

#[test]
fn test_optional_parameter_type_from_default() {
    // Optional parameter with String default should have String type
    let source = r#"
def greet(name = "World")
  name.upcase
end
"#;

    let (genv, _lenv) = analyze(source);

    // No type errors - name is String from default value, upcase exists on String
    assert_eq!(genv.type_errors.len(), 0);
}

#[test]
fn test_optional_parameter_type_error() {
    // Optional parameter with Integer default should error on String method
    let source = r#"
def greet(count = 42)
  count.upcase
end
"#;

    let (genv, _lenv) = analyze(source);

    // Type error should be detected: count is Integer, upcase is not available
    assert_eq!(genv.type_errors.len(), 1);
    assert_eq!(genv.type_errors[0].method_name, "upcase");
}

#[test]
fn test_mixed_required_and_optional_parameters() {
    let source = r#"
def greet(greeting, name = "World")
  x = greeting
  y = name.upcase
end
"#;

    let (genv, _lenv) = analyze(source);

    // No type errors - name has String type from default
    assert_eq!(genv.type_errors.len(), 0);
}

#[test]
fn test_rest_parameter_has_array_type() {
    let source = r#"
def collect(*items)
  x = items
end
"#;

    let (genv, _lenv) = analyze(source);

    // No type errors - items is Array
    assert_eq!(genv.type_errors.len(), 0);
}

#[test]
fn test_keyword_rest_parameter_has_hash_type() {
    let source = r#"
def configure(**options)
  x = options
end
"#;

    let (genv, _lenv) = analyze(source);

    // No type errors - options is Hash
    assert_eq!(genv.type_errors.len(), 0);
}

#[test]
fn test_all_parameter_types_combined() {
    let source = r#"
def complex_method(required, optional = "default", *rest, **kwargs)
  a = required
  b = optional.upcase
  c = rest
  d = kwargs
end
"#;

    let (genv, _lenv) = analyze(source);

    // No type errors - optional.upcase should work (String has upcase)
    assert_eq!(genv.type_errors.len(), 0);
}
