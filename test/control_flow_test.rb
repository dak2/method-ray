# frozen_string_literal: true

require 'test_helper'

class ControlFlowTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # No Error
  # ============================================

  def test_next_in_while_loop
    source = <<~RUBY
      class Foo
        def bar
          while true
            next
          end
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_next_with_value
    source = <<~RUBY
      class Foo
        def bar
          while true
            next 42
          end
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_break_in_while_loop
    source = <<~RUBY
      class Foo
        def bar
          while true
            break
          end
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_break_with_value
    source = <<~RUBY
      class Foo
        def bar
          while true
            break 42
          end
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_redo_in_while_loop
    source = <<~RUBY
      class Foo
        def bar
          while true
            redo
          end
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_retry_in_rescue
    source = <<~RUBY
      class Foo
        def bar
          begin
            "hello"
          rescue
            retry
          end
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_next_in_block
    source = <<~RUBY
      class Foo
        def bar
          [1, 2, 3].each do |x|
            next if true
            x.even?
          end
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_next_value_type_checked
    source = <<~RUBY
      class Foo
        def bar
          42
        end

        def baz
          while true
            next self.bar.upcase
          end
        end
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end

  def test_break_value_type_checked
    source = <<~RUBY
      class Foo
        def bar
          42
        end

        def baz
          while true
            break self.bar.upcase
          end
        end
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end
end
