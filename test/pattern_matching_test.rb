# frozen_string_literal: true

require 'test_helper'

class PatternMatchingTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # No Error
  # ============================================

  def test_literal_pattern_return_type
    source = <<~RUBY
      val = 42
      result = case val
      in 1
        "one"
      in 2
        "two"
      else
        "other"
      end
      result.upcase
    RUBY
    assert_no_check_errors(source)
  end

  def test_capture_pattern_integer
    source = <<~RUBY
      case 42
      in Integer => n
        n.even?
      end
    RUBY
    assert_no_check_errors(source)
  end

  def test_array_pattern_binding
    source = <<~RUBY
      case [1, "hello"]
      in [x, y]
        x
      end
    RUBY
    assert_no_check_errors(source)
  end

  def test_hash_pattern_binding
    source = <<~RUBY
      case { name: "Alice", age: 30 }
      in { name:, age: }
        name
      end
    RUBY
    assert_no_check_errors(source)
  end

  def test_pattern_match_with_else
    source = <<~RUBY
      result = case 42
      in Integer => n
        n.to_s
      else
        "none"
      end
      result.upcase
    RUBY
    assert_no_check_errors(source)
  end

  def test_splat_pattern
    source = <<~RUBY
      case [1, 2, 3]
      in [first, *rest]
        rest.length
      end
    RUBY
    assert_no_check_errors(source)
  end

  def test_find_pattern
    source = <<~RUBY
      case [1, 2, 3, 4, 5]
      in [*pre, 3, *post]
        pre.length
        post.length
      end
    RUBY
    assert_no_check_errors(source)
  end

  def test_alternation_pattern
    source = <<~RUBY
      case 42
      in 1 | 2 | 3
        "small"
      in Integer => n
        n.to_s
      end
    RUBY
    assert_no_check_errors(source)
  end

  def test_pinned_variable_pattern
    source = <<~RUBY
      expected = 42
      case 42
      in ^expected
        "matched"
      end
    RUBY
    assert_no_check_errors(source)
  end

  def test_guard_if_pattern
    source = <<~RUBY
      case 42
      in Integer => n if n > 0
        n.to_s
      end
    RUBY
    assert_no_check_errors(source)
  end

  def test_hash_splat_pattern
    source = <<~RUBY
      case { name: "Alice", age: 30 }
      in { name:, **rest }
        rest.keys
      end
    RUBY
    assert_no_check_errors(source)
  end

  def test_variable_binding_pattern
    source = <<~RUBY
      case 42
      in x
        x
      end
    RUBY
    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_capture_pattern_type_error
    source = <<~RUBY
      case 42
      in Integer => n
        n.upcase
      end
    RUBY
    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end

  def test_capture_pattern_string_type_error
    source = <<~RUBY
      case "hello"
      in String => s
        s.even?
      end
    RUBY
    assert_check_error(source, method_name: 'even?', receiver_type: 'String')
  end

  def test_hash_splat_type_error
    source = <<~RUBY
      case { name: "Alice" }
      in { **rest }
        rest.upcase
      end
    RUBY
    assert_check_error(source, method_name: 'upcase', receiver_type: 'Hash')
  end

  def test_splat_type_error
    source = <<~RUBY
      case [1, 2, 3]
      in [first, *rest]
        rest.upcase
      end
    RUBY
    assert_check_error(source, method_name: 'upcase', receiver_type: 'Array')
  end
end
