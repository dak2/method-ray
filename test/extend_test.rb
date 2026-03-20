# frozen_string_literal: true

require 'test_helper'

class ExtendTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # No Error
  # ============================================

  def test_extend_basic_no_error
    source = <<~RUBY
      module ClassMethods
        def find(id)
          "found"
        end
      end

      class User
        extend ClassMethods
      end

      User.find(1)
    RUBY

    assert_no_check_errors(source)
  end

  def test_extend_multiple_modules
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
        extend A, B
      end

      User.a_method
      User.b_method
    RUBY

    assert_no_check_errors(source)
  end

  def test_extend_qualified_module
    source = <<~RUBY
      module Api
        module ClassHelpers
          def search(query)
            "results"
          end
        end
      end

      class User
        extend Api::ClassHelpers
      end

      User.search("test")
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Type Inference
  # ============================================

  def test_extend_simultaneous_mro_order
    source = <<~RUBY
      module A
        def foo
          "from A"
        end
      end

      module B
        def foo
          42
        end
      end

      class User
        extend A, B
      end

      x = User.foo
    RUBY

    assert_type(source, 'x', 'String')
  end

  def test_extend_def_self_takes_priority
    source = <<~RUBY
      module ClassMethods
        def find(id)
          "found"
        end
      end

      class User
        extend ClassMethods

        def self.find(id)
          42
        end
      end

      x = User.find(1)
    RUBY

    assert_type(source, 'x', 'Integer')
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_extend_does_not_affect_instance
    source = <<~RUBY
      module ClassMethods
        def find(id)
          "found"
        end
      end

      class User
        extend ClassMethods
      end

      User.new.find(1)
    RUBY

    assert_check_error(source, method_name: 'find', receiver_type: 'User')
  end
end
