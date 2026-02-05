//! Integration Tests - End-to-end analyzer tests
//!
//! This module contains integration tests that verify:
//! - Class/method definition handling
//! - Instance variable type tracking across methods
//! - Type error detection for undefined methods
//! - Method chain type inference

use crate::analyzer::AstInstaller;
use crate::env::{GlobalEnv, LocalEnv};
use crate::parser::ParseSession;
use crate::types::Type;

/// Helper to run analysis on Ruby source code
fn analyze(source: &str) -> (GlobalEnv, LocalEnv) {
    let session = ParseSession::new();
    let parse_result = session.parse_source(source, "test.rb").unwrap();

    let mut genv = GlobalEnv::new();

    // Register common methods
    genv.register_builtin_method(Type::string(), "upcase", Type::string());
    genv.register_builtin_method(Type::string(), "downcase", Type::string());

    // Register Float methods
    genv.register_builtin_method(Type::float(), "to_s", Type::string());
    genv.register_builtin_method(Type::float(), "to_i", Type::integer());
    genv.register_builtin_method(Type::float(), "round", Type::integer());
    genv.register_builtin_method(Type::float(), "ceil", Type::integer());
    genv.register_builtin_method(Type::float(), "floor", Type::integer());
    genv.register_builtin_method(Type::float(), "abs", Type::float());

    // Register iterator methods for block tests
    genv.register_builtin_method(Type::array(), "each", Type::array());
    genv.register_builtin_method(Type::array(), "map", Type::array());
    genv.register_builtin_method(Type::hash(), "each", Type::hash());

    // Register Regexp methods
    genv.register_builtin_method(Type::regexp(), "match", Type::instance("MatchData"));
    genv.register_builtin_method(Type::regexp(), "match?", Type::instance("TrueClass"));
    genv.register_builtin_method(Type::regexp(), "source", Type::string());

    // Register Range methods
    genv.register_builtin_method(Type::range(), "to_a", Type::array());
    genv.register_builtin_method(Type::range(), "size", Type::integer());
    genv.register_builtin_method(Type::range(), "count", Type::integer());
    genv.register_builtin_method(Type::range(), "first", Type::Bot);
    genv.register_builtin_method(Type::range(), "last", Type::Bot);
    genv.register_builtin_method(Type::range(), "include?", Type::instance("TrueClass"));
    genv.register_builtin_method(Type::range(), "cover?", Type::instance("TrueClass"));

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

// ============================================
// Block Tests
// ============================================

#[test]
fn test_block_parameter_available_as_local_var() {
    let source = r#"
x = [1, 2, 3]
x.each { |item| y = item }
"#;

    let (genv, _lenv) = analyze(source);

    // No type errors should occur - item parameter should be available in block
    assert_eq!(genv.type_errors.len(), 0);
}

#[test]
fn test_block_with_multiple_parameters() {
    let source = r#"
x = { a: 1, b: 2 }
x.each { |key, value| a = key; b = value }
"#;

    let (genv, _lenv) = analyze(source);

    // No type errors should occur - both parameters should be available
    assert_eq!(genv.type_errors.len(), 0);
}

#[test]
fn test_block_do_end_syntax() {
    let source = r#"
x = [1, 2, 3]
x.map do |item|
  y = item
end
"#;

    let (genv, _lenv) = analyze(source);

    // No type errors should occur - do...end blocks work the same as { }
    assert_eq!(genv.type_errors.len(), 0);
}

#[test]
fn test_block_accesses_outer_scope_variable() {
    let source = r#"
outer = "hello"
x = [1, 2, 3]
x.each { |item| y = outer.upcase }
"#;

    let (genv, _lenv) = analyze(source);

    // No type errors - block can access outer scope variable
    assert_eq!(genv.type_errors.len(), 0);
}

#[test]
fn test_nested_blocks() {
    let source = r#"
x = [[1, 2], [3, 4]]
x.each { |row| row.each { |item| y = item } }
"#;

    let (genv, _lenv) = analyze(source);

    // No type errors should occur - nested blocks work correctly
    assert_eq!(genv.type_errors.len(), 0);
}

#[test]
fn test_block_in_method_definition() {
    let source = r#"
def process_items
  items = [1, 2, 3]
  items.each { |item| x = item }
end
"#;

    let (genv, _lenv) = analyze(source);

    // No type errors should occur - blocks work inside methods
    assert_eq!(genv.type_errors.len(), 0);
}

#[test]
fn test_block_in_class_method() {
    let source = r#"
class Processor
  def process
    items = [1, 2, 3]
    items.map { |item| item }
  end
end
"#;

    let (genv, _lenv) = analyze(source);

    // No type errors should occur
    assert_eq!(genv.type_errors.len(), 0);
}

// ============================================
// Float Literal Tests
// ============================================

#[test]
fn test_float_literal_basic() {
    let source = r#"x = 3.14"#;

    let (genv, lenv) = analyze(source);

    let x_vtx = lenv.get_var("x").unwrap();
    assert_eq!(genv.get_vertex(x_vtx).unwrap().show(), "Float");
}

#[test]
fn test_float_literal_type_error() {
    let source = r#"
class Calculator
  def compute
    x = 3.14
    y = x.upcase
  end
end
"#;

    let (genv, _lenv) = analyze(source);

    // Type error should be detected: Float doesn't have upcase method
    assert_eq!(genv.type_errors.len(), 1);
    assert_eq!(genv.type_errors[0].method_name, "upcase");
}

#[test]
fn test_float_specific_methods() {
    let source = r#"
x = 3.14
a = x.ceil
b = x.floor
c = x.abs
"#;

    let (genv, lenv) = analyze(source);

    // No type errors - ceil, floor, abs are valid Float methods
    assert_eq!(genv.type_errors.len(), 0);

    // ceil and floor return Integer
    let a_vtx = lenv.get_var("a").unwrap();
    assert_eq!(genv.get_vertex(a_vtx).unwrap().show(), "Integer");

    let b_vtx = lenv.get_var("b").unwrap();
    assert_eq!(genv.get_vertex(b_vtx).unwrap().show(), "Integer");

    // abs returns Float
    let c_vtx = lenv.get_var("c").unwrap();
    assert_eq!(genv.get_vertex(c_vtx).unwrap().show(), "Float");
}

// ============================================
// Regexp Literal Tests
// ============================================

#[test]
fn test_regexp_literal_basic() {
    let source = r#"x = /hello/"#;

    let (genv, lenv) = analyze(source);

    let x_vtx = lenv.get_var("x").unwrap();
    assert_eq!(genv.get_vertex(x_vtx).unwrap().show(), "Regexp");
}

#[test]
fn test_regexp_literal_type_error() {
    let source = r#"
class Matcher
  def find
    x = /pattern/
    y = x.upcase
  end
end
"#;

    let (genv, _lenv) = analyze(source);

    // Type error should be detected: Regexp doesn't have upcase method
    assert_eq!(genv.type_errors.len(), 1);
    assert_eq!(genv.type_errors[0].method_name, "upcase");
}

#[test]
fn test_regexp_specific_methods() {
    let source = r#"
x = /hello/
a = x.source
"#;

    let (genv, lenv) = analyze(source);

    // No type errors - source is a valid Regexp method
    assert_eq!(genv.type_errors.len(), 0);

    // source returns String
    let a_vtx = lenv.get_var("a").unwrap();
    assert_eq!(genv.get_vertex(a_vtx).unwrap().show(), "String");
}

// ============================================
// Range Literal Tests
// ============================================

#[test]
fn test_range_literal_basic() {
    let source = r#"x = 1..5"#;

    let (genv, lenv) = analyze(source);

    let x_vtx = lenv.get_var("x").unwrap();
    assert_eq!(genv.get_vertex(x_vtx).unwrap().show(), "Range[Integer]");
}

#[test]
fn test_range_literal_exclusive() {
    let source = r#"x = 1...5"#;

    let (genv, lenv) = analyze(source);

    let x_vtx = lenv.get_var("x").unwrap();
    assert_eq!(genv.get_vertex(x_vtx).unwrap().show(), "Range[Integer]");
}

#[test]
fn test_range_literal_string() {
    let source = r#"x = "a".."z""#;

    let (genv, lenv) = analyze(source);

    let x_vtx = lenv.get_var("x").unwrap();
    assert_eq!(genv.get_vertex(x_vtx).unwrap().show(), "Range[String]");
}

#[test]
fn test_range_generic_float() {
    let source = r#"x = 1.0..5.0"#;

    let (genv, lenv) = analyze(source);

    let x_vtx = lenv.get_var("x").unwrap();
    assert_eq!(genv.get_vertex(x_vtx).unwrap().show(), "Range[Float]");
}

// ============================================
// Nested Generic Array Tests
// ============================================

#[test]
fn test_nested_array_integer() {
    let source = r#"x = [[1, 2], [3]]"#;

    let (genv, lenv) = analyze(source);

    let x_vtx = lenv.get_var("x").unwrap();
    assert_eq!(
        genv.get_vertex(x_vtx).unwrap().show(),
        "Array[Array[Integer]]"
    );
}

#[test]
fn test_deeply_nested_array() {
    let source = r#"x = [[[1]]]"#;

    let (genv, lenv) = analyze(source);

    let x_vtx = lenv.get_var("x").unwrap();
    assert_eq!(
        genv.get_vertex(x_vtx).unwrap().show(),
        "Array[Array[Array[Integer]]]"
    );
}

#[test]
fn test_nested_array_mixed() {
    let source = r#"x = [[1], ["a"]]"#;

    let (genv, lenv) = analyze(source);

    let x_vtx = lenv.get_var("x").unwrap();
    // Union type order may vary, so check for either order
    let result = genv.get_vertex(x_vtx).unwrap().show();
    assert!(
        result == "Array[Array[Integer] | Array[String]]"
            || result == "Array[Array[String] | Array[Integer]]",
        "Expected nested array with union, got: {}",
        result
    );
}

#[test]
fn test_range_literal_type_error() {
    let source = r#"
class Calculator
  def compute
    x = 1..10
    y = x.upcase
  end
end
"#;

    let (genv, _lenv) = analyze(source);

    assert_eq!(genv.type_errors.len(), 1);
    assert_eq!(genv.type_errors[0].method_name, "upcase");
}

#[test]
fn test_range_specific_methods() {
    let source = r#"
x = 1..10
a = x.to_a
b = x.size
"#;

    let (genv, lenv) = analyze(source);

    assert_eq!(genv.type_errors.len(), 0);

    let a_vtx = lenv.get_var("a").unwrap();
    assert_eq!(genv.get_vertex(a_vtx).unwrap().show(), "Array");

    let b_vtx = lenv.get_var("b").unwrap();
    assert_eq!(genv.get_vertex(b_vtx).unwrap().show(), "Integer");
}

// ============================================
// Hash Generic Type Tests
// ============================================

#[test]
fn test_hash_symbol_integer() {
    let source = r#"x = { a: 1, b: 2 }"#;

    let (genv, lenv) = analyze(source);

    let x_vtx = lenv.get_var("x").unwrap();
    assert_eq!(
        genv.get_vertex(x_vtx).unwrap().show(),
        "Hash[Symbol, Integer]"
    );
}

#[test]
fn test_hash_string_string() {
    let source = r#"x = { "k" => "v" }"#;

    let (genv, lenv) = analyze(source);

    let x_vtx = lenv.get_var("x").unwrap();
    assert_eq!(
        genv.get_vertex(x_vtx).unwrap().show(),
        "Hash[String, String]"
    );
}

#[test]
fn test_hash_mixed_values() {
    let source = r#"x = { a: 1, b: "x" }"#;

    let (genv, lenv) = analyze(source);

    let x_vtx = lenv.get_var("x").unwrap();
    let result = genv.get_vertex(x_vtx).unwrap().show();
    // Union type order may vary
    assert!(
        result == "Hash[Symbol, Integer | String]"
            || result == "Hash[Symbol, String | Integer]",
        "Expected Hash with union value type, got: {}",
        result
    );
}

#[test]
fn test_hash_empty() {
    let source = r#"x = {}"#;

    let (genv, lenv) = analyze(source);

    let x_vtx = lenv.get_var("x").unwrap();
    assert_eq!(genv.get_vertex(x_vtx).unwrap().show(), "Hash");
}

#[test]
fn test_hash_nested() {
    let source = r#"x = { a: [1] }"#;

    let (genv, lenv) = analyze(source);

    let x_vtx = lenv.get_var("x").unwrap();
    assert_eq!(
        genv.get_vertex(x_vtx).unwrap().show(),
        "Hash[Symbol, Array[Integer]]"
    );
}

// ============================================
// Error Location Tests
// ============================================

#[test]
fn test_error_location_points_to_dot() {
    // Test that error location points to the dot operator, not the receiver
    // "name.abs" - error should point to column 5 (the `.`)
    let source = "name = \"x\"\nname.abs";

    let (genv, _lenv) = analyze(source);

    assert_eq!(genv.type_errors.len(), 1);
    let error = &genv.type_errors[0];

    // Location should be present
    assert!(error.location.is_some());
    let loc = error.location.as_ref().unwrap();

    // Line 2, column 5 (1-indexed) - the dot position
    assert_eq!(loc.line, 2);
    assert_eq!(loc.column, 5); // "name" is 4 chars, "." is at column 5
}

#[test]
fn test_error_location_method_chain() {
    // Test error location in method chain: "x.upcase.foo"
    // Error should point to the second dot (for undefined method `foo`)
    let source = "x = \"hello\"\ny = x.upcase.foo";

    let (genv, _lenv) = analyze(source);

    assert_eq!(genv.type_errors.len(), 1);
    let error = &genv.type_errors[0];
    assert_eq!(error.method_name, "foo");

    assert!(error.location.is_some());
    let loc = error.location.as_ref().unwrap();

    // Line 2, the second dot position
    assert_eq!(loc.line, 2);
    // "y = x.upcase" is 12 chars, "." is at column 13
    assert_eq!(loc.column, 13);
}
