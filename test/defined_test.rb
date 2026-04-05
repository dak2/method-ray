# frozen_string_literal: true

require 'test_helper'

class DefinedTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # No Error
  # ============================================

  def test_defined_basic
    source = <<~RUBY
      result = defined?(foo)
    RUBY
    assert_no_check_errors(source)
  end

  def test_defined_with_method_call
    source = <<~RUBY
      defined?(some_method)
    RUBY
    assert_no_check_errors(source)
  end

  def test_defined_child_not_evaluated
    source = <<~RUBY
      defined?(42.upcase)
    RUBY
    assert_no_check_errors(source)
  end

  def test_defined_with_constant
    source = <<~RUBY
      defined?(CONSTANT)
    RUBY
    assert_no_check_errors(source)
  end

  def test_defined_with_instance_variable
    source = <<~RUBY
      defined?(@var)
    RUBY
    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_defined_result_string_method_error
    source = <<~RUBY
      class Foo
        def bar
          result = defined?(foo)
          result.length
        end
      end
    RUBY
    assert_check_error(source, method_name: 'length', receiver_type: 'nil')
  end

  def test_defined_result_integer_method_error
    source = <<~RUBY
      class Foo
        def bar
          result = defined?(foo)
          result.even?
        end
      end
    RUBY
    assert_check_error(source, method_name: 'even?', receiver_type: 'String')
  end
end
