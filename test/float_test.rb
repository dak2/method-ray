# frozen_string_literal: true

require 'test_helper'

class FloatTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # Type Inference
  # ============================================

  def test_float_literal
    assert_type "x = 3.14", "x", "Float"
  end

  def test_float_ceil_returns_integer
    types = infer("x = 3.14\na = x.ceil")
    assert_equal "Integer", types["a"]
  end

  def test_float_floor_returns_integer
    types = infer("x = 3.14\nb = x.floor")
    assert_equal "Integer", types["b"]
  end

  def test_float_abs_returns_float
    types = infer("x = 3.14\nc = x.abs")
    assert_equal "Float", types["c"]
  end

  # ============================================
  # No Error
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

  # ============================================
  # Error Detection
  # ============================================

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
end
