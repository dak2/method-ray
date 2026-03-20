# frozen_string_literal: true

require 'test_helper'

class LoopTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # No Error (check CLI)
  # ============================================

  def test_while_loop_no_false_positive
    source = <<~RUBY
      class Foo
        def bar
          while true
            "hello".upcase
          end
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_until_loop_no_false_positive
    source = <<~RUBY
      class Foo
        def bar
          until false
            "hello".upcase
          end
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_for_loop_basic_no_error
    source = <<~RUBY
      class Foo
        def bar
          for item in ["hello", "world"]
            item.upcase
          end
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_while_modifier_form_no_error
    source = <<~RUBY
      class Foo
        def bar
          "hello".upcase while false
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_begin_end_while_no_error
    source = <<~RUBY
      class Foo
        def bar
          begin
            "hello".upcase
          end while false
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_for_empty_body_no_error
    source = <<~RUBY
      class Foo
        def bar
          for x in [1, 2, 3]
          end
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection (check CLI)
  # ============================================

  def test_while_loop_detects_type_error
    source = <<~RUBY
      class Foo
        def bar
          42
        end

        def baz
          while true
            self.bar.upcase
          end
        end
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end

  def test_for_loop_detects_type_error
    source = <<~RUBY
      class Foo
        def bar
          for item in [1, 2, 3]
            item.upcase
          end
        end
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end

  def test_until_loop_detects_type_error
    source = <<~RUBY
      class Foo
        def bar
          42
        end

        def baz
          until false
            self.bar.upcase
          end
        end
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end
end
