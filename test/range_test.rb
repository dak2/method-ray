# frozen_string_literal: true

require 'test_helper'

class RangeTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # Type Inference
  # ============================================

  def test_range_integer
    assert_type "x = 1..5", "x", "Range[Integer]"
  end

  def test_range_exclusive
    assert_type "x = 1...5", "x", "Range[Integer]"
  end

  def test_range_string
    assert_type 'x = "a".."z"', "x", "Range[String]"
  end

  def test_range_float
    assert_type "x = 1.0..5.0", "x", "Range[Float]"
  end

  def test_range_to_a_returns_array
    types = infer("x = 1..10\na = x.to_a")
    assert_equal "Array[Elem]", types["a"]
  end

  def test_range_size_returns_integer
    types = infer("x = 1..10\nb = x.size")
    assert_equal "Integer | Float | nil", types["b"]
  end

  # ============================================
  # No Error
  # ============================================

  def test_range_methods_no_error
    source = <<~RUBY
      x = 1..10
      a = x.to_a
      b = x.size
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

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
end
