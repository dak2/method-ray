# frozen_string_literal: true

require 'test_helper'

class ParameterTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # No Error
  # ============================================

  def test_method_parameter_available_as_local_var
    source = <<~RUBY
      def greet(name)
        x = name
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_method_multiple_parameters
    source = <<~RUBY
      def calculate(a, b, c)
        x = a
        y = b
        z = c
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_class_method_with_parameter
    source = <<~RUBY
      class User
        def initialize(name)
          @name = name
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_parameter_method_call_bot_type
    source = <<~RUBY
      def greet(name)
        name.upcase
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_optional_parameter_type_from_default
    source = <<~RUBY
      def greet(name = "World")
        name.upcase
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_mixed_required_and_optional_parameters
    source = <<~RUBY
      def greet(greeting, name = "World")
        x = greeting
        y = name.upcase
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_rest_parameter_has_array_type
    source = <<~RUBY
      def collect(*items)
        x = items
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_keyword_rest_parameter_has_hash_type
    source = <<~RUBY
      def configure(**options)
        x = options
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_all_parameter_types_combined
    source = <<~RUBY
      def complex_method(required, optional = "default", *rest, **kwargs)
        a = required
        b = optional.upcase
        c = rest
        d = kwargs
      end
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_optional_parameter_type_error
    source = <<~RUBY
      def greet(count = 42)
        count.upcase
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end
end
