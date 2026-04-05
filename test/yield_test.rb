# frozen_string_literal: true

require 'test_helper'

class YieldTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # No Error
  # ============================================

  def test_yield_with_argument_no_error
    source = <<~RUBY
      class Foo
        def run
          yield "done"
        end
      end

      Foo.new.run { |msg| msg.upcase }
    RUBY

    assert_no_check_errors(source)
  end

  def test_yield_without_argument_no_error
    source = <<~RUBY
      class Foo
        def notify
          yield
        end
      end

      Foo.new.notify { "notified" }
    RUBY

    assert_no_check_errors(source)
  end

  def test_yield_with_multiple_arguments_no_error
    source = <<~RUBY
      class Foo
        def pair
          yield "key", 1
        end
      end

      Foo.new.pair { |k, v| k.upcase }
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_yield_argument_type_error
    source = <<~RUBY
      class Foo
        def bar
          yield 42.upcase
        end
      end

      Foo.new.bar { |msg| msg.to_s }
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end
end
