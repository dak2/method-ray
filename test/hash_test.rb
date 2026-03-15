# frozen_string_literal: true

require 'test_helper'

class HashTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # Type Inference
  # ============================================

  def test_hash_symbol_integer
    assert_type 'x = { a: 1, b: 2 }', 'x', 'Hash[Symbol, Integer]'
  end

  def test_hash_string_string
    assert_type 'x = { "k" => "v" }', 'x', 'Hash[String, String]'
  end

  def test_hash_mixed_values
    types = infer('x = { a: 1, b: "x" }')
    assert_includes(
      ['Hash[Symbol, Integer | String]', 'Hash[Symbol, String | Integer]'],
      types['x']
    )
  end

  def test_hash_empty
    assert_type 'x = {}', 'x', 'Hash'
  end

  def test_hash_nested
    assert_type 'x = { a: [1] }', 'x', 'Hash[Symbol, Array[Integer]]'
  end

  # ============================================
  # No Error
  # ============================================

  def test_hash_methods_no_error
    source = <<~RUBY
      x = { a: 1, b: 2 }
      a = x.keys
      b = x.values
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_hash_type_error
    source = <<~RUBY
      class Config
        def load
          x = { a: 1 }
          y = x.upcase
        end
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Hash')
  end
end
