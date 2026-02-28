# frozen_string_literal: true

require 'test_helper'

class ObjectTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # No Error
  # ============================================

  def test_nil_check_on_string
    source = <<~RUBY
      class Checker
        def check
          x = "hello"
          x.nil?
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_nil_check_on_integer
    source = <<~RUBY
      class Checker
        def check
          x = 42
          x.nil?
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_class_method_on_string
    source = <<~RUBY
      class Inspector
        def inspect_type
          x = "hello"
          x.class
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_dup_on_string
    source = <<~RUBY
      class Copier
        def copy
          x = "hello"
          x.dup
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_freeze_on_string
    source = <<~RUBY
      class Freezer
        def freeze_value
          x = "hello"
          x.freeze
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_frozen_check_on_array
    source = <<~RUBY
      class Checker
        def check
          x = [1, 2, 3]
          x.frozen?
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_is_a_on_string
    source = <<~RUBY
      class TypeChecker
        def check
          x = "hello"
          x.is_a?(String)
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_respond_to_on_integer
    source = <<~RUBY
      class Checker
        def check
          x = 42
          x.respond_to?(:even?)
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_to_s_on_integer
    source = <<~RUBY
      class Converter
        def convert
          x = 42
          x.to_s
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_hash_method_on_string
    source = <<~RUBY
      class Hasher
        def compute
          x = "hello"
          x.hash
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_equal_check_on_integer
    source = <<~RUBY
      class Comparer
        def compare
          x = 42
          x.equal?(42)
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_object_methods_on_hash
    source = <<~RUBY
      class Processor
        def process
          x = { a: 1 }
          x.nil?
          x.frozen?
          x.class
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Fallback (user-defined class → Object)
  # ============================================

  def test_nil_check_on_user_defined_class
    source = <<~RUBY
      class Account
        def valid?
          other = Account.new
          other.nil?
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_class_on_user_defined_class
    source = <<~RUBY
      class Entity
        def type_name
          other = Entity.new
          other.class
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_to_s_on_user_defined_class
    source = <<~RUBY
      class Label
        def display
          other = Label.new
          other.to_s
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Override
  # ============================================

  def test_override_to_s_no_error
    source = <<~RUBY
      class Foo
        def to_s
          "Foo"
        end

        def bar
          to_s.upcase
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_override_nil_no_error
    source = <<~RUBY
      class Foo
        def nil?
          false
        end

        def bar
          nil?
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection (Override)
  # ============================================

  def test_override_to_s_return_type_error
    source = <<~RUBY
      class Foo
        def to_s
          "Foo"
        end

        def bar
          to_s.even?
        end
      end
    RUBY

    assert_check_error(source, method_name: 'even?', receiver_type: 'String')
  end

  def test_override_dup_return_type_error
    source = <<~RUBY
      class Foo
        def dup
          "copy"
        end

        def bar
          dup.even?
        end
      end
    RUBY

    assert_check_error(source, method_name: 'even?', receiver_type: 'String')
  end
end
