# frozen_string_literal: true

require 'test_helper'

class OperatorTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # Type Inference (infer_types API)
  # ============================================

  def test_and_operator_union_type
    source = <<~RUBY
      x = true && "hello"
    RUBY

    types = infer(source)
    type_str = types["x"]
    assert_includes type_str, "TrueClass"
    assert_includes type_str, "String"
  end

  def test_or_operator_union_type
    source = <<~RUBY
      x = 42 || "hello"
    RUBY

    types = infer(source)
    type_str = types["x"]
    assert_includes type_str, "Integer"
    assert_includes type_str, "String"
  end

  def test_arithmetic_operator_type
    source = <<~RUBY
      x = 1 + 2
    RUBY

    assert_type source, "x", "Integer"
  end

  # ============================================
  # No Error (check CLI)
  # ============================================

  def test_and_operator_no_error
    source = <<~RUBY
      class Foo
        def bar
          "a" && "b"
        end

        def baz
          self.bar.upcase
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_or_operator_no_error
    source = <<~RUBY
      class Foo
        def bar
          "a" || "b"
        end

        def baz
          self.bar.upcase
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_arithmetic_operator_no_error
    source = <<~RUBY
      class Foo
        def bar
          1 + 2
        end

        def baz
          self.bar.even?
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection (check CLI)
  # ============================================

  def test_and_operator_type_error
    source = <<~RUBY
      class Foo
        def bar
          "a" && "b"
        end

        def baz
          self.bar.even?
        end
      end
    RUBY

    assert_check_error(source, method_name: 'even?', receiver_type: 'String')
  end

  def test_or_operator_type_error
    source = <<~RUBY
      class Foo
        def bar
          "a" || "b"
        end

        def baz
          self.bar.even?
        end
      end
    RUBY

    assert_check_error(source, method_name: 'even?', receiver_type: 'String')
  end
end
