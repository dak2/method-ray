# frozen_string_literal: true

require 'test_helper'

class InstanceVariableTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # No Error
  # ============================================

  def test_class_with_instance_variable
    source = <<~RUBY
      class User
        def initialize
          @name = "John"
        end

        def greet
          @name.upcase
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_instance_variable_type_error
    source = <<~RUBY
      class User
        def initialize
          @name = 123
        end

        def greet
          @name.upcase
        end
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end
end
