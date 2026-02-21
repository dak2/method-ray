# frozen_string_literal: true

require 'test_helper'

class ParenthesesTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # Type Inference
  # ============================================

  def test_parenthesized_integer
    assert_type 'x = (42)', "x", "Integer"
  end

  # ============================================
  # No Error
  # ============================================

  def test_parenthesized_string_method_call_no_error
    source = <<~RUBY
      x = ("hello")
      y = x.upcase
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_parenthesized_string_type_error
    source = <<~RUBY
      class Formatter
        def format
          x = ("hello")
          y = x.ceil
        end
      end
    RUBY

    assert_check_error(source, method_name: 'ceil', receiver_type: 'String')
  end
end
