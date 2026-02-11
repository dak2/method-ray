# frozen_string_literal: true

require 'test_helper'

class StringTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # Type Inference
  # ============================================

  def test_string_literal
    assert_type 'x = "hello"', "x", "String"
  end

  def test_multiple_vars
    types = infer(<<~RUBY)
      x = "hello"
      y = 42
    RUBY

    assert_equal "String", types["x"]
    assert_equal "Integer", types["y"]
  end

  def test_method_call_return_type
    types = infer(<<~RUBY)
      x = "hello"
      y = x.upcase
    RUBY

    assert_equal "String", types["x"]
    assert_equal "String", types["y"]
  end

  # ============================================
  # No Error
  # ============================================

  def test_method_chain_no_error
    source = <<~RUBY
      x = "hello"
      y = x.upcase.downcase
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_string_type_error
    source = <<~RUBY
      class Formatter
        def format
          x = "hello"
          y = x.ceil
        end
      end
    RUBY

    assert_check_error(source, method_name: 'ceil', receiver_type: 'String')
  end
end
