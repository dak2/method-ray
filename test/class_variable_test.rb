# frozen_string_literal: true

require 'test_helper'

class ClassVariableTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # No Error
  # ============================================

  def test_class_variable_basic
    source = <<~RUBY
      class Counter
        @@count = 0

        def increment
          @@count = 1
        end

        def value
          @@count.to_s
        end
      end
    RUBY
    assert_no_check_errors(source)
  end

  def test_class_variable_in_method
    source = <<~RUBY
      class Logger
        def setup
          @@prefix = "LOG"
        end

        def log(msg)
          @@prefix.upcase
        end
      end
    RUBY
    assert_no_check_errors(source)
  end

  def test_class_variable_write_before_read
    source = <<~RUBY
      class Config
        def initialize
          @@default = "production"
        end

        def env
          @@default.upcase
        end
      end
    RUBY
    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_class_variable_type_error
    source = <<~RUBY
      class Counter
        def setup
          @@count = 42
        end

        def display
          @@count.upcase
        end
      end
    RUBY
    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end

  def test_class_variable_type_error_in_same_method
    source = <<~RUBY
      class Broken
        def run
          @@value = 100
          @@value.length
        end
      end
    RUBY
    assert_check_error(source, method_name: 'length', receiver_type: 'Integer')
  end

  def test_class_variable_class_body_write_type_error
    source = <<~RUBY
      class Counter
        @@count = 42

        def display
          @@count.upcase
        end
      end
    RUBY
    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end

  def test_class_variable_isolation_between_classes
    source = <<~RUBY
      class StringHolder
        def setup
          @@value = "hello"
        end

        def run
          @@value.upcase
        end
      end

      class IntHolder
        def setup
          @@value = 42
        end

        def run
          @@value.upcase
        end
      end
    RUBY
    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end

  def test_class_variable_in_module_no_crash
    source = <<~RUBY
      module Config
        @@setting = "production"
      end
    RUBY
    # Module-scoped @@var is not yet supported (v0.2.0 limitation).
    # The checker should not crash.
    assert_no_check_errors(source)
  end
end
