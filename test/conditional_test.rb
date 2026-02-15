# frozen_string_literal: true

require 'test_helper'

class ConditionalTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # Type Inference (infer_types API)
  # ============================================

  def test_if_else_union_type
    source = <<~RUBY
      x = if true
            "hello"
          else
            42
          end
    RUBY

    types = infer(source)
    type_str = types["x"]
    assert_includes type_str, "Integer"
    assert_includes type_str, "String"
  end

  def test_if_without_else_includes_nil
    source = <<~RUBY
      x = if true
            "hello"
          end
    RUBY

    types = infer(source)
    type_str = types["x"]
    assert_includes type_str, "String"
    assert_includes type_str, "nil"
  end

  def test_if_elsif_else_union
    source = <<~RUBY
      x = if true
            "hello"
          elsif false
            42
          else
            :sym
          end
    RUBY

    types = infer(source)
    type_str = types["x"]
    assert_includes type_str, "String"
    assert_includes type_str, "Integer"
    assert_includes type_str, "Symbol"
  end

  def test_unless_else_union_type
    source = <<~RUBY
      x = unless true
            "a"
          else
            1
          end
    RUBY

    types = infer(source)
    type_str = types["x"]
    assert_includes type_str, "String"
    assert_includes type_str, "Integer"
  end

  def test_unless_without_else_includes_nil
    source = <<~RUBY
      x = unless true
            "hello"
          end
    RUBY

    types = infer(source)
    type_str = types["x"]
    assert_includes type_str, "String"
    assert_includes type_str, "nil"
  end

  def test_case_when_else_union
    source = <<~RUBY
      x = case :status
          when :active
            "active"
          when :inactive
            42
          else
            :fallback
          end
    RUBY

    types = infer(source)
    type_str = types["x"]
    assert_includes type_str, "String"
    assert_includes type_str, "Integer"
    assert_includes type_str, "Symbol"
  end

  def test_case_without_else_includes_nil
    source = <<~RUBY
      x = case :status
          when :active
            "active"
          end
    RUBY

    types = infer(source)
    type_str = types["x"]
    assert_includes type_str, "String"
    assert_includes type_str, "nil"
  end

  def test_same_type_branches
    source = <<~RUBY
      x = if true
            "hello"
          else
            "world"
          end
    RUBY

    assert_type source, "x", "String"
  end

  # ============================================
  # No Error (check CLI)
  # ============================================

  def test_if_else_string_branch_upcase
    source = <<~RUBY
      class Formatter
        def format
          if true
            "hello"
          else
            "world"
          end
        end

        def run
          self.format.upcase
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_conditional_return_used_with_valid_method
    source = <<~RUBY
      class Converter
        def convert
          if true
            "text"
          else
            "other"
          end
        end

        def process
          self.convert.length
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection (check CLI)
  # ============================================

  def test_if_else_string_branch_even_error
    source = <<~RUBY
      class Foo
        def bar
          if true
            "hello"
          else
            "world"
          end
        end

        def baz
          self.bar.even?
        end
      end
    RUBY

    assert_check_error(source, method_name: 'even?', receiver_type: 'String')
  end

  def test_conditional_integer_branch_upcase_error
    source = <<~RUBY
      class Foo
        def bar
          if true
            42
          else
            99
          end
        end

        def baz
          self.bar.upcase
        end
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end
end
