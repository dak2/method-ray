# frozen_string_literal: true

require 'test_helper'

class ExceptionTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # Type Inference (infer_types API)
  # ============================================

  def test_begin_rescue_union_type
    source = <<~RUBY
      x = begin
            "hello"
          rescue
            42
          end
    RUBY

    types = infer(source)
    type_str = types["x"]
    assert_includes type_str, "Integer"
    assert_includes type_str, "String"
  end

  def test_begin_rescue_else_excludes_begin_body
    source = <<~RUBY
      x = begin
            "hello"
          rescue
            42
          else
            :ok
          end
    RUBY

    types = infer(source)
    type_str = types["x"]
    assert_includes type_str, "Symbol"
    assert_includes type_str, "Integer"
    refute_includes type_str, "String"
  end

  def test_rescue_variable_typed_as_specific_exception
    source = <<~RUBY
      x = begin
            "hello"
          rescue ArgumentError => e
            e
          end
    RUBY

    types = infer(source)
    type_str = types["x"]
    assert_includes type_str, "ArgumentError"
  end

  def test_rescue_variable_typed_as_union_of_exceptions
    source = <<~RUBY
      x = begin
            "hello"
          rescue TypeError, NameError => e
            e
          end
    RUBY

    types = infer(source)
    type_str = types["x"]
    assert_includes type_str, "TypeError"
    assert_includes type_str, "NameError"
  end

  def test_rescue_without_exception_class_defaults_to_standard_error
    source = <<~RUBY
      x = begin
            "hello"
          rescue => e
            e
          end
    RUBY

    types = infer(source)
    type_str = types["x"]
    assert_includes type_str, "StandardError"
  end

  def test_rescue_modifier_union_type
    source = <<~RUBY
      x = "hello" rescue 42
    RUBY

    types = infer(source)
    type_str = types["x"]
    assert_includes type_str, "Integer"
    assert_includes type_str, "String"
  end

  # ============================================
  # No Error (check CLI)
  # ============================================

  def test_begin_rescue_no_false_positive
    source = <<~RUBY
      class Converter
        def convert
          begin
            "result"
          rescue
            "fallback"
          end
        end

        def run
          self.convert.upcase
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_inline_rescue_no_false_positive
    source = <<~RUBY
      class Converter
        def convert
          "result" rescue "fallback"
        end

        def run
          self.convert.upcase
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection (check CLI)
  # ============================================

  def test_rescue_with_specific_exception_returns_string
    source = <<~RUBY
      class Catcher
        def catch_it
          begin
            "result"
          rescue ArgumentError
            "fallback"
          end
        end

        def run
          self.catch_it.upcase
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_rescue_branch_type_error
    source = <<~RUBY
      class Foo
        def bar
          begin
            "hello"
          rescue
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
end
