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

  # ============================================
  # Block Return Type Propagation
  # ============================================

  def test_map_propagates_block_return_type
    source = <<~RUBY
      class Formatter
        def format
          result = [1, 2, 3].map { |x| x.to_s }
          result.first.upcase
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_map_with_integer_return
    source = <<~RUBY
      class Counter
        def count
          result = ["a", "b"].map { |x| x.length }
          result.first.even?
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_map_block_return_type_error
    source = <<~RUBY
      class Formatter
        def format
          result = [1, 2, 3].map { |x| x.to_s }
          result.first.even?
        end
      end
    RUBY

    assert_check_error(source, method_name: 'even?', receiver_type: 'String')
  end

  def test_each_does_not_propagate_block_return_type
    source = <<~RUBY
      class Processor
        def process
          [1, 2, 3].each { |x| x.to_s }
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_empty_block_body
    source = <<~RUBY
      class Processor
        def process
          [1, 2, 3].map { |x| }
        end
      end
    RUBY

    assert_no_check_errors(source)
  end
end
