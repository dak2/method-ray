# frozen_string_literal: true

require 'test_helper'

class ArrayTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # Type Inference
  # ============================================

  def test_nested_array_integer
    assert_type "x = [[1, 2], [3]]", "x", "Array[Array[Integer]]"
  end

  def test_deeply_nested_array
    assert_type "x = [[[1]]]", "x", "Array[Array[Array[Integer]]]"
  end

  def test_nested_array_mixed
    types = infer('x = [[1], ["a"]]')
    assert_includes(
      ["Array[Array[Integer] | Array[String]]", "Array[Array[String] | Array[Integer]]"],
      types["x"]
    )
  end

  # ============================================
  # No Error
  # ============================================

  def test_array_methods_no_error
    source = <<~RUBY
      x = [1, 2, 3]
      a = x.length
      b = x.first
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_array_type_error
    source = <<~RUBY
      class Processor
        def process
          x = [1, 2, 3]
          y = x.upcase
        end
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Array')
  end
end
