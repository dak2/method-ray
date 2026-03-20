# frozen_string_literal: true

require 'test_helper'

class ReturnTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # No Error (check CLI)
  # ============================================

  def test_return_string_upcase_no_error
    source = <<~RUBY
      class Formatter
        def format
          return "" if true
          "default"
        end

        def run
          self.format.upcase
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_return_only_method_no_error
    source = <<~RUBY
      class Foo
        def bar
          return "hello"
        end

        def baz
          self.bar.upcase
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_no_return_backward_compat
    source = <<~RUBY
      class Foo
        def bar
          "hello"
        end

        def baz
          self.bar.upcase
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_multiple_returns_union_no_error
    source = <<~RUBY
      class Converter
        def convert
          return "hello" if true
          return "world" if false
          "default"
        end

        def run
          self.convert.upcase
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_return_with_implicit_return_union_no_error
    source = <<~RUBY
      class Calculator
        def compute
          return 0 if true
          42
        end

        def run
          self.compute.even?
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Type Inference (infer_types API)
  # ============================================

  def test_return_dead_code_over_approximation
    source = <<~RUBY
      class Foo
        def bar
          return "hello"
          42
        end

        def baz
          self.bar
        end
      end

      x = Foo.new.baz
    RUBY

    types = infer(source)
    type_str = types['x']
    # Dead code after return is still processed (over-approximation)
    assert_includes type_str, 'Integer'
    assert_includes type_str, 'String'
  end

  # ============================================
  # Error Detection (check CLI)
  # ============================================

  def test_return_string_even_error
    source = <<~RUBY
      class Parser
        def parse
          return "error"
        end

        def run
          self.parse.even?
        end
      end
    RUBY

    assert_check_error(source, method_name: 'even?', receiver_type: 'String')
  end

  def test_return_union_string_integer_even_error
    source = <<~RUBY
      class Validator
        def validate
          return "invalid" if true
          42
        end

        def run
          self.validate.even?
        end
      end
    RUBY

    assert_check_error(source, method_name: 'even?', receiver_type: 'String')
  end
end
