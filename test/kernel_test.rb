# frozen_string_literal: true

require 'test_helper'

class KernelTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # No Error
  # ============================================

  def test_freeze_on_integer
    source = <<~RUBY
      class Freezer
        def freeze_value
          x = 42
          x.freeze
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_frozen_on_string
    source = <<~RUBY
      class Checker
        def check
          x = "hello"
          x.frozen?
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_tap_on_string
    source = <<~RUBY
      class Logger
        def log
          x = "hello"
          x.tap
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_object_id_on_string
    source = <<~RUBY
      class Inspector
        def inspect_id
          x = "hello"
          x.object_id
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_send_on_string
    source = <<~RUBY
      class Caller
        def dynamic_call
          x = "hello"
          x.send(:upcase)
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_kernel_methods_on_array
    source = <<~RUBY
      class Processor
        def process
          x = [1, 2, 3]
          x.freeze
          x.frozen?
          x.object_id
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_kernel_methods_on_hash
    source = <<~RUBY
      class Processor
        def process
          x = { a: 1 }
          x.freeze
          x.frozen?
          x.object_id
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Override
  # ============================================

  def test_override_freeze_no_error
    source = <<~RUBY
      class Foo
        def freeze
          self
        end

        def bar
          freeze
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_override_hash_no_error
    source = <<~RUBY
      class Foo
        def hash
          42
        end

        def bar
          hash.even?
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection (Override)
  # ============================================

  def test_override_freeze_return_type_error
    source = <<~RUBY
      class Foo
        def freeze
          self
        end

        def bar
          freeze.baz
        end
      end
    RUBY

    assert_check_error(source, method_name: 'baz', receiver_type: 'Foo')
  end

  def test_override_hash_return_type_error
    source = <<~RUBY
      class Foo
        def hash
          42
        end

        def bar
          hash.upcase
        end
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end
end
