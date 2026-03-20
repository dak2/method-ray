# frozen_string_literal: true

require 'test_helper'

class SuperTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # No Error
  # ============================================

  def test_super_basic_no_error
    source = <<~RUBY
      class Animal
        def speak
          "..."
        end
      end

      class Dog < Animal
        def speak
          super
        end
      end

      Dog.new.speak
    RUBY

    assert_no_check_errors(source)
  end

  def test_super_with_method_chain
    source = <<~RUBY
      class Animal
        def speak
          "hello"
        end
      end

      class Dog < Animal
        def speak
          super.upcase
        end
      end

      Dog.new.speak
    RUBY

    assert_no_check_errors(source)
  end

  def test_super_with_arguments
    source = <<~RUBY
      class Base
        def greet(name)
          name
        end
      end

      class Child < Base
        def greet(name)
          super(name)
        end
      end

      Child.new.greet("Alice")
    RUBY

    assert_no_check_errors(source)
  end

  def test_super_multi_level_inheritance
    source = <<~RUBY
      class A
        def foo
          "hello"
        end
      end

      class B < A
        def foo
          super
        end
      end

      class C < B
        def foo
          super
        end
      end

      C.new.foo
    RUBY

    assert_no_check_errors(source)
  end

  def test_super_explicit_empty_args
    source = <<~RUBY
      class Animal
        def speak
          "hello"
        end
      end

      class Dog < Animal
        def speak
          super()
        end
      end

      Dog.new.speak
    RUBY

    assert_no_check_errors(source)
  end

  def test_super_qualified_superclass
    source = <<~RUBY
      module Animals
        class Pet
          def name
            "pet"
          end
        end
      end

      class Dog < Animals::Pet
        def name
          super
        end
      end

      Dog.new.name
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_super_method_chain_type_error
    source = <<~RUBY
      class Animal
        def speak
          "hello"
        end
      end

      class Dog < Animal
        def speak
          super.even?
        end
      end

      Dog.new.speak
    RUBY

    assert_check_error(source, method_name: 'even?', receiver_type: 'String')
  end
end
