# frozen_string_literal: true

require 'test_helper'

class GlobalVariableTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # No Error
  # ============================================

  def test_global_variable_basic
    source = <<~RUBY
      $config = "production"
      $config.upcase
    RUBY
    assert_no_check_errors(source)
  end

  def test_global_variable_type_error
    source = <<~RUBY
      $count = 42
      $count.upcase
    RUBY
    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end

  def test_global_variable_in_method
    source = <<~RUBY
      class App
        def setup
          $logger = "Logger instance"
        end

        def run
          $logger.upcase
        end
      end
    RUBY
    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_global_variable_across_methods_type_error
    source = <<~RUBY
      class App
        def setup
          $counter = 0
        end

        def process
          $counter.upcase
        end
      end
    RUBY
    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end

  def test_global_variable_top_level
    source = <<~RUBY
      $name = "Alice"
      $age = 30

      def greet
        $name.upcase
      end

      def show_age
        $age.upcase
      end
    RUBY
    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end

  def test_global_variable_type_error_in_same_method
    source = <<~RUBY
      def run
        $value = 100
        $value.upcase
      end
    RUBY
    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end

  def test_global_variable_across_classes
    source = <<~RUBY
      class Writer
        def write
          $shared = "data"
        end
      end

      class Reader
        def read
          $shared.upcase
        end
      end
    RUBY
    assert_no_check_errors(source)
  end

  def test_multiple_global_variables_independent
    source = <<~RUBY
      $name = "Alice"
      $count = 42
      $name.upcase
      $count.to_s
    RUBY
    assert_no_check_errors(source)
  end
end
