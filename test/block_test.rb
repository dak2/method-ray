# frozen_string_literal: true

require 'test_helper'

class BlockTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # No Error
  # ============================================

  def test_block_parameter_available_as_local_var
    source = <<~RUBY
      x = [1, 2, 3]
      x.each { |item| y = item }
    RUBY

    assert_no_check_errors(source)
  end

  def test_block_with_multiple_parameters
    source = <<~RUBY
      x = { a: 1, b: 2 }
      x.each { |key, value| a = key; b = value }
    RUBY

    assert_no_check_errors(source)
  end

  def test_block_do_end_syntax
    source = <<~RUBY
      x = [1, 2, 3]
      x.map do |item|
        y = item
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_block_accesses_outer_scope_variable
    source = <<~RUBY
      outer = "hello"
      x = [1, 2, 3]
      x.each { |item| y = outer.upcase }
    RUBY

    assert_no_check_errors(source)
  end

  def test_nested_blocks
    source = <<~RUBY
      x = [[1, 2], [3, 4]]
      x.each { |row| row.each { |item| y = item } }
    RUBY

    assert_no_check_errors(source)
  end

  def test_block_in_method_definition
    source = <<~RUBY
      def process_items
        items = [1, 2, 3]
        items.each { |item| x = item }
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_block_in_class_method
    source = <<~RUBY
      class Processor
        def process
          items = [1, 2, 3]
          items.map { |item| item }
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_block_parameter_from_each_char
    source = <<~RUBY
      class Foo
        def bar
          "hello".each_char { |c| c.upcase }
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_block_parameter_from_each_char_type_error
    source = <<~RUBY
      class Foo
        def bar
          "hello".each_char { |c| c.even? }
        end
      end
    RUBY

    assert_check_error(source, method_name: 'even?', receiver_type: 'String')
  end

  def test_block_body_does_not_affect_method_return
    source = <<~RUBY
      class Foo
        def bar
          [1, 2].each { |x| "string" }
        end

        def baz
          self.bar.upcase
        end
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'self')
  end

  def test_block_outer_variable_type_error
    source = <<~RUBY
      class Processor
        def process
          x = 123
          items = [1, 2, 3]
          items.each { |item| x.upcase }
        end
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end
end
