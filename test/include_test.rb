# frozen_string_literal: true

require 'test_helper'

class IncludeTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # No Error
  # ============================================

  def test_include_basic_no_error
    source = <<~RUBY
      module Greetable
        def greet
          "Hello!"
        end
      end

      class User
        include Greetable
      end

      User.new.greet
    RUBY

    assert_no_check_errors(source)
  end

  def test_include_receiverless_call
    source = <<~RUBY
      module Greetable
        def greet
          "Hello!"
        end
      end

      class User
        include Greetable

        def say_hello
          greet
        end
      end

      User.new.say_hello
    RUBY

    assert_no_check_errors(source)
  end

  def test_include_multiple_modules
    source = <<~RUBY
      module A
        def a_method
          "a"
        end
      end

      module B
        def b_method
          42
        end
      end

      class User
        include A
        include B
      end

      User.new.a_method
      User.new.b_method
    RUBY

    assert_no_check_errors(source)
  end

  def test_include_simultaneous
    source = <<~RUBY
      module A
        def a_method
          "a"
        end
      end

      module B
        def b_method
          42
        end
      end

      class User
        include A, B
      end

      User.new.a_method
      User.new.b_method
    RUBY

    assert_no_check_errors(source)
  end

  def test_include_qualified_module
    source = <<~RUBY
      module Api
        module Helpers
          def help
            "help"
          end
        end
      end

      class User
        include Api::Helpers
      end

      User.new.help
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_include_method_type_error
    source = <<~RUBY
      module Greetable
        def greet
          "Hello!"
        end
      end

      class User
        include Greetable
      end

      User.new.greet.even?
    RUBY

    assert_check_error(source, method_name: 'even?', receiver_type: 'String')
  end
end
