# frozen_string_literal: true

require 'test_helper'

class CompoundAssignmentTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # No Error
  # ============================================

  def test_local_operator_assign_no_error
    source = <<~RUBY
      class Foo
        def bar
          x = 1
          x += 2
          x.even?
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_local_or_assign_no_error
    source = <<~RUBY
      class Foo
        def bar
          x = "hello"
          x ||= "world"
          x.upcase
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_local_or_assign_no_error_without_initialization
    source = <<~RUBY
      class Foo
        def bar
          x ||= "world"
          x.upcase
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_local_and_assign_no_error
    source = <<~RUBY
      class Foo
        def bar
          x = "hello"
          x &&= "world"
          x.upcase
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_ivar_operator_assign_no_error
    source = <<~RUBY
      class Counter
        def initialize
          @count = 0
        end

        def increment
          @count += 1
        end

        def value
          @count.even?
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_ivar_or_assign_no_error
    source = <<~RUBY
      class Config
        def initialize
          @name = "default"
        end

        def set_name
          @name ||= "fallback"
        end

        def display
          @name.upcase
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_operator_assign_type_error
    source = <<~RUBY
      class Foo
        def bar
          x = 1
          x += 2
          x.upcase
        end
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end

  def test_or_assign_type_error
    source = <<~RUBY
      class Foo
        def bar
          x = 1
          x ||= "hello"
          x.even?
        end
      end
    RUBY

    assert_check_error(source, method_name: 'even?', receiver_type: 'String')
  end

  def test_and_assign_type_error
    source = <<~RUBY
      class Foo
        def bar
          x &&= "hello"
          x.even?
        end
      end
    RUBY

    assert_check_error(source, method_name: 'even?', receiver_type: 'String')
  end

  def test_ivar_operator_assign_type_error
    source = <<~RUBY
      class Counter
        def initialize
          @count = 0
        end

        def increment
          @count += 1
        end

        def name
          @count.upcase
        end
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end
end
