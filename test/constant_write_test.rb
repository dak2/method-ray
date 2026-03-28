# frozen_string_literal: true

require 'test_helper'

class ConstantWriteTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # No Error
  # ============================================

  def test_constant_basic_string
    source = <<~RUBY
      MESSAGE = "hello"
      MESSAGE.upcase
    RUBY
    assert_no_check_errors(source)
  end

  def test_constant_basic_integer
    source = <<~RUBY
      MAX_SIZE = 100
      MAX_SIZE.even?
    RUBY
    assert_no_check_errors(source)
  end

  def test_constant_type_error
    source = <<~RUBY
      MAX_SIZE = 100
      MAX_SIZE.upcase
    RUBY
    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end

  def test_constant_in_class
    source = <<~RUBY
      class Config
        MAX_RETRIES = 3

        def retry_count
          MAX_RETRIES.to_s
        end
      end
    RUBY
    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_constant_type_error_in_class_method
    source = <<~RUBY
      class Config
        MAX_RETRIES = 3

        def check
          MAX_RETRIES.upcase
        end
      end
    RUBY
    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end

  def test_top_level_constant_from_class
    source = <<~RUBY
      MAX = 100

      class Foo
        def limit
          MAX.to_s
        end
      end
    RUBY
    assert_no_check_errors(source)
  end

  def test_multiple_independent_constants
    source = <<~RUBY
      MAX = 100
      NAME = "Alice"
      MAX.even?
      NAME.upcase
    RUBY
    assert_no_check_errors(source)
  end
end
