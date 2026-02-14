# frozen_string_literal: true

require 'test_helper'

class ImplicitSelfCallTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # No Error
  # ============================================

  def test_receiverless_method_call
    source = <<~RUBY
      class User
        def name
          "Alice"
        end

        def greet
          name.upcase
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_receiverless_method_call_with_arguments
    source = <<~RUBY
      class Greeter
        def greet(name)
          name.upcase
        end

        def run
          greet("Alice")
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_receiverless_call_chain
    source = <<~RUBY
      class Formatter
        def title
          "hello"
        end

        def format
          title.upcase.downcase
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_receiverless_call_in_nested_class
    source = <<~RUBY
      module Api
        class User
          def name
            "Alice"
          end

          def greet
            name.upcase
          end
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_attr_reader_does_not_panic
    source = <<~RUBY
      class User
        attr_reader :name

        def greet
          "hello"
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_super_does_not_panic
    source = <<~RUBY
      class Base
        def greet
          "hello"
        end
      end

      class Child < Base
        def greet
          super
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_param_type_propagation_via_implicit_self
    source = <<~RUBY
      class Calculator
        def add(x, y)
          x.even?
        end

        def compute
          add(1, 2)
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_receiverless_method_return_type_error
    source = <<~RUBY
      class User
        def name
          "Alice"
        end

        def greet
          name.even?
        end
      end
    RUBY

    assert_check_error(source, method_name: 'even?', receiver_type: 'String')
  end

  def test_receiverless_method_return_integer_type_error
    source = <<~RUBY
      class Calculator
        def answer
          42
        end

        def compute
          answer.upcase
        end
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end

  def test_param_type_propagation_error_via_implicit_self
    source = <<~RUBY
      class Processor
        def process(value)
          value.upcase
        end

        def run
          process(42)
        end
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end
end
