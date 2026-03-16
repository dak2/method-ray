# frozen_string_literal: true

require 'test_helper'

class InheritanceTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # No Error
  # ============================================

  def test_inheritance_basic_no_error
    source = <<~RUBY
      class Animal
        def speak
          "..."
        end
      end

      class Dog < Animal
      end

      Dog.new.speak
    RUBY

    assert_no_check_errors(source)
  end

  def test_inheritance_multi_level
    source = <<~RUBY
      class Animal
        def speak
          "..."
        end
      end

      class Dog < Animal
      end

      class Puppy < Dog
      end

      Puppy.new.speak
    RUBY

    assert_no_check_errors(source)
  end

  def test_inheritance_override
    source = <<~RUBY
      class Animal
        def speak
          "generic"
        end
      end

      class Dog < Animal
        def speak
          42
        end
      end

      Dog.new.speak
    RUBY

    assert_no_check_errors(source)
  end

  def test_inheritance_child_and_parent_include_mro
    source = <<~RUBY
      module Swimmable
        def move
          "swim"
        end
      end

      module Runnable
        def move
          "run"
        end
      end

      class Animal
        include Swimmable
      end

      class Dog < Animal
        include Runnable
      end

      Dog.new.move.upcase
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_inheritance_undefined_method
    source = <<~RUBY
      class Animal
        def speak
          "..."
        end
      end

      class Dog < Animal
      end

      Dog.new.fly
    RUBY

    assert_check_error(source, method_name: 'fly', receiver_type: 'Dog')
  end

  def test_inheritance_method_chain_type_error
    source = <<~RUBY
      class Animal
        def speak
          "hello"
        end
      end

      class Dog < Animal
      end

      Dog.new.speak.even?
    RUBY

    assert_check_error(source, method_name: 'even?', receiver_type: 'String')
  end
end
