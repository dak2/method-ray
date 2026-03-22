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
  # Parameter Type Propagation from Call Site
  # ============================================

  def test_param_type_propagation_string
    source = <<~RUBY
      class Greeter
        def greet(name)
          name.upcase
        end

        def run
          self.greet("Alice")
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_param_type_propagation_integer
    source = <<~RUBY
      class Calculator
        def double(n)
          n.even?
        end

        def run
          self.double(42)
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_param_type_propagation_multiple_params
    source = <<~RUBY
      class Formatter
        def format(greeting, name)
          greeting.upcase
          name.downcase
        end

        def run
          self.format("Hello", "World")
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_param_type_propagation_chain
    source = <<~RUBY
      class Validator
        def valid?(str)
          str.length
        end

        def check
          self.valid?("test")
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Keyword Argument via .new → initialize
  # ============================================

  def test_keyword_arg_via_new_to_initialize
    source = <<~RUBY
      class Config
        def initialize(debug:)
          @debug = debug
        end

        def debug?
          @debug
        end
      end

      x = Config.new(debug: true).debug?
    RUBY

    assert_type(source, 'x', 'TrueClass')
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

  def test_param_type_propagation_error
    source = <<~RUBY
      class Processor
        def process(value)
          value.upcase
        end

        def run
          self.process(42)
        end
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end
end
