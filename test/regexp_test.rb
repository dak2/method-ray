# frozen_string_literal: true

require 'test_helper'

class RegexpTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # Type Inference
  # ============================================

  def test_regexp_literal
    assert_type "x = /hello/", "x", "Regexp"
  end

  def test_regexp_source_returns_string
    types = infer("x = /hello/\na = x.source")
    assert_equal "String", types["a"]
  end

  # ============================================
  # No Error
  # ============================================

  def test_regexp_methods_no_error
    source = <<~RUBY
      x = /hello/
      a = x.source
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

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

  def test_interpolated_regexp_type_error
    source = <<~RUBY
      class Formatter
        def format
          x = /hello \#{1}/
          y = x.ceil
        end
      end
    RUBY
    assert_check_error(source, method_name: 'ceil', receiver_type: 'Regexp')
  end
end
