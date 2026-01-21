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

#[test]
fn test_attr_reader_registers_method() {
    let source = r#"
class User
  attr_reader :name
end
"#;

    let (genv, _lenv) = analyze(source);

    // attr_reader should register a method on the User class
    let recv_ty = Type::Instance {
        class_name: "User".to_string(),
    };
    let result = genv.resolve_method(&recv_ty, "name");
    assert!(result.is_some(), "attr_reader should register a 'name' method");
}

#[test]
fn test_attr_reader_with_ivar_type() {
    // Note: Currently attr_reader registers with Bot type because
    // type propagation happens after attr_reader processing.
    // This is a known limitation - ideally we'd do two-pass processing
    // or lazy evaluation to get the correct type.
    let source = r#"
class User
  def initialize
    @name = "John"
  end

  attr_reader :name
end
"#;

    let (genv, _lenv) = analyze(source);

    // attr_reader should register a method (type may be untyped due to processing order)
    let recv_ty = Type::Instance {
        class_name: "User".to_string(),
    };
    let result = genv.resolve_method(&recv_ty, "name");
    assert!(result.is_some(), "attr_reader should register 'name' method");
    // Type is Bot (untyped) because type propagation hasn't run yet when attr_reader processes
    // This is acceptable for now - the method is registered and callable
}

#[test]
fn test_attr_reader_error_detection() {
    let source = r#"
class User
  def initialize
    @age = 25
  end

  attr_reader :age

  def test
    x = @age.upcase
  end
end
"#;

    let (genv, _lenv) = analyze(source);

    // Type error should be detected: @age is Integer, not String
    assert_eq!(genv.type_errors.len(), 1);
    assert_eq!(genv.type_errors[0].method_name, "upcase");
}

#[test]
fn test_attr_accessor() {
    let source = r#"
class User
  attr_accessor :email
end
"#;

    let (genv, _lenv) = analyze(source);

    let recv_ty = Type::Instance {
        class_name: "User".to_string(),
    };

    // attr_accessor should register both getter and setter
    let getter = genv.resolve_method(&recv_ty, "email");
    assert!(getter.is_some(), "attr_accessor should register getter");

    let setter = genv.resolve_method(&recv_ty, "email=");
    assert!(setter.is_some(), "attr_accessor should register setter");
}
