# frozen_string_literal: true

require 'test_helper'

class CheckTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # Error Detection Tests (7)
  # ============================================

  def test_class_method_error_detection
    source = <<~RUBY
      class User
        def test
          x = 123
          y = x.upcase
        end
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end

  def test_instance_variable_type_error
    source = <<~RUBY
      class User
        def initialize
          @name = 123
        end

        def greet
          @name.upcase
        end
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end

  def test_multiple_classes
    source = <<~RUBY
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
    RUBY

    stdout, _stderr, status = run_check(source)

    refute status.success?
    assert_match(/undefined method `upcase` for Integer/, stdout)
    refute_match(/Post/, stdout)
  end

  def test_float_literal_type_error
    source = <<~RUBY
      class Calculator
        def compute
          x = 3.14
          y = x.upcase
        end
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Float')
  end

  def test_regexp_literal_type_error
    source = <<~RUBY
      class Matcher
        def find
          x = /pattern/
          y = x.upcase
        end
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Regexp')
  end

  def test_optional_parameter_type_error
    source = <<~RUBY
      def greet(count = 42)
        count.upcase
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end

  def test_range_literal_type_error
    source = <<~RUBY
      class Calculator
        def compute
          x = 1..10
          y = x.upcase
        end
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Range')
  end

  # ============================================
  # No Error Tests - Instance Variables (1)
  # ============================================

  def test_class_with_instance_variable
    source = <<~RUBY
      class User
        def initialize
          @name = "John"
        end

        def greet
          @name.upcase
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # No Error Tests - Method Parameters (8)
  # ============================================

  def test_method_parameter_available_as_local_var
    source = <<~RUBY
      def greet(name)
        x = name
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_method_multiple_parameters
    source = <<~RUBY
      def calculate(a, b, c)
        x = a
        y = b
        z = c
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_class_method_with_parameter
    source = <<~RUBY
      class User
        def initialize(name)
          @name = name
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_parameter_method_call_bot_type
    source = <<~RUBY
      def greet(name)
        name.upcase
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_optional_parameter_type_from_default
    source = <<~RUBY
      def greet(name = "World")
        name.upcase
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_mixed_required_and_optional_parameters
    source = <<~RUBY
      def greet(greeting, name = "World")
        x = greeting
        y = name.upcase
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_rest_parameter_has_array_type
    source = <<~RUBY
      def collect(*items)
        x = items
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_keyword_rest_parameter_has_hash_type
    source = <<~RUBY
      def configure(**options)
        x = options
      end
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # No Error Tests - Blocks (7)
  # ============================================

  def test_block_parameter_available_as_local_var
    source = <<~RUBY
      x = [1, 2, 3]
      x.each { |item| y = item }
    RUBY

    assert_no_check_errors(source)
  end

  def test_block_with_multiple_parameters
    source = <<~RUBY
      x = { a: 1, b: 2 }
      x.each { |key, value| a = key; b = value }
    RUBY

    assert_no_check_errors(source)
  end

  def test_block_do_end_syntax
    source = <<~RUBY
      x = [1, 2, 3]
      x.map do |item|
        y = item
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_block_accesses_outer_scope_variable
    source = <<~RUBY
      outer = "hello"
      x = [1, 2, 3]
      x.each { |item| y = outer.upcase }
    RUBY

    assert_no_check_errors(source)
  end

  def test_nested_blocks
    source = <<~RUBY
      x = [[1, 2], [3, 4]]
      x.each { |row| row.each { |item| y = item } }
    RUBY

    assert_no_check_errors(source)
  end

  def test_block_in_method_definition
    source = <<~RUBY
      def process_items
        items = [1, 2, 3]
        items.each { |item| x = item }
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_block_in_class_method
    source = <<~RUBY
      class Processor
        def process
          items = [1, 2, 3]
          items.map { |item| item }
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # No Error Tests - Method Calls (3)
  # ============================================

  def test_float_methods_no_error
    source = <<~RUBY
      x = 3.14
      a = x.ceil
      b = x.floor
      c = x.abs
    RUBY

    assert_no_check_errors(source)
  end

  def test_regexp_methods_no_error
    source = <<~RUBY
      x = /hello/
      a = x.source
    RUBY

    assert_no_check_errors(source)
  end

  def test_range_methods_no_error
    source = <<~RUBY
      x = 1..10
      a = x.to_a
      b = x.size
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # No Error Tests - Method Chains (2)
  # ============================================

  def test_method_chain_no_error
    source = <<~RUBY
      x = "hello"
      y = x.upcase.downcase
    RUBY

    assert_no_check_errors(source)
  end

  def test_all_parameter_types_combined
    source = <<~RUBY
      def complex_method(required, optional = "default", *rest, **kwargs)
        a = required
        b = optional.upcase
        c = rest
        d = kwargs
      end
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Location Tests (2)
  # ============================================

  def test_error_location_points_to_dot
    source = "name = \"x\"\nname.abs"

    assert_error_at(source, line: 2, column: 5)
  end

  def test_error_location_method_chain
    source = "x = \"hello\"\ny = x.upcase.foo"

    assert_error_at(source, line: 2, column: 13)
  end
end
