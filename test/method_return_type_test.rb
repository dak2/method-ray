# frozen_string_literal: true

require 'test_helper'

class MethodReturnTypeTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # No Error
  # ============================================

  def test_user_method_returns_string
    source = <<~RUBY
      class User
        def name
          "Alice"
        end

        def greet
          self.name.upcase
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_user_method_returns_integer
    source = <<~RUBY
      class Calculator
        def answer
          42
        end

        def compute
          self.answer.even?
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_user_method_chain
    source = <<~RUBY
      class Formatter
        def title
          "hello"
        end

        def format
          self.title.upcase.downcase
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_user_method_return_type_error
    source = <<~RUBY
      class User
        def name
          "Alice"
        end

        def greet
          self.name.even?
        end
      end
    RUBY

    assert_check_error(source, method_name: 'even?', receiver_type: 'String')
  end

  def test_user_method_returns_integer_type_error
    source = <<~RUBY
      class Calculator
        def answer
          42
        end

        def compute
          self.answer.upcase
        end
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end
end
