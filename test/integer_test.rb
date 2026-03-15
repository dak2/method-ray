# frozen_string_literal: true

require 'test_helper'

class IntegerTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # Type Inference
  # ============================================

  def test_integer_literal
    assert_type 'x = 42', 'x', 'Integer'
  end

  # ============================================
  # No Error
  # ============================================

  def test_integer_methods_no_error
    source = <<~RUBY
      x = 42
      a = x.abs
      b = x.to_f
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_class_method_error_detection
    source = <<~RUBY
      class User
        def test
          x = 123
          y = x.upcase
        end
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end

  def test_multiple_classes
    source = <<~RUBY
      class User
        def name
          x = 123
          x.upcase
        end
      end

      class Post
        def title
          y = "hello"
          y.upcase
        end
      end
    RUBY

    stdout, _stderr, status = run_check(source)

    refute status.success?
    assert_match(/undefined method `upcase` for Integer/, stdout)
    refute_match(/Post/, stdout)
  end
end
