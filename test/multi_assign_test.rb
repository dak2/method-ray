# frozen_string_literal: true

require 'test_helper'

class MultiAssignTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # Type Inference
  # ============================================

  def test_basic_multi_assign
    types = infer('a, b = 1, "hello"')
    assert_equal 'Integer', types['a']
    assert_equal 'String', types['b']
  end

  def test_lhs_longer_than_rhs
    types = infer('a, b, c = 1, 2')
    assert_equal 'Integer', types['a']
    assert_equal 'Integer', types['b']
    assert_equal 'nil', types['c']
  end

  def test_splat_basic
    types = infer('first, *rest = 1, 2, 3')
    assert_equal 'Integer', types['first']
    assert_equal 'Array[Integer]', types['rest']
  end

  def test_splat_with_rights
    types = infer('first, *rest, last = 1, 2, 3, 4')
    assert_equal 'Integer', types['first']
    assert_equal 'Array[Integer]', types['rest']
    assert_equal 'Integer', types['last']
  end

  def test_splat_rights_no_lefts
    types = infer('*rest, last = 1, 2, 3')
    assert_equal 'Array[Integer]', types['rest']
    assert_equal 'Integer', types['last']
  end

  def test_splat_empty
    types = infer('first, *rest = 1')
    assert_equal 'Integer', types['first']
    assert_equal 'Array[untyped]', types['rest']
  end

  def test_splat_lefts_exceed_rhs
    types = infer('a, b, c, *rest = 1, 2')
    assert_equal 'Integer', types['a']
    assert_equal 'Integer', types['b']
    assert_equal 'nil', types['c']
    assert_equal 'Array[untyped]', types['rest']
  end

  def test_splat_with_rights_insufficient_rhs
    types = infer('a, *rest, z = 1')
    assert_equal 'Integer', types['a']
    assert_equal 'Array[untyped]', types['rest']
    assert_equal 'nil', types['z']
  end

  def test_scalar_rhs
    types = infer('a, b = 42')
    assert_equal 'Integer', types['a']
    assert_equal 'nil', types['b']
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_multi_assign_type_error
    source = <<~RUBY
      a, b = 1, 2
      a.upcase
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end
end
