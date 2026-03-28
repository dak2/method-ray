# frozen_string_literal: true

require 'test_helper'

class LambdaTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # No Error
  # ============================================

  def test_lambda_basic
    source = <<~RUBY
      f = -> { 42 }
      result = f.call
    RUBY
    assert_no_check_errors(source)
  end

  def test_lambda_with_args
    source = <<~RUBY
      f = ->(x) { x * 2 }
      result = f.call(21)
    RUBY
    assert_no_check_errors(source)
  end

  def test_lambda_method_basic
    source = <<~RUBY
      f = lambda { 42 }
      result = f.call
    RUBY
    assert_no_check_errors(source)
  end

  def test_proc_basic
    source = <<~RUBY
      f = proc { 42 }
      result = f.call
    RUBY
    assert_no_check_errors(source)
  end

  def test_proc_new_basic
    source = <<~RUBY
      f = Proc.new { 42 }
      result = f.call
    RUBY
    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_lambda_body_type_error
    source = <<~RUBY
      f = -> { 42.upcase }
    RUBY
    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end

  def test_lambda_with_args_body_type_error
    source = <<~RUBY
      f = ->(x) { x.upcase }
      f.call(42)
    RUBY
    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end

  def test_lambda_call_return_type_error
    source = <<~RUBY
      f = -> { 42 }
      f.call.upcase
    RUBY
    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end

  def test_lambda_method_body_type_error
    source = <<~RUBY
      f = lambda { 42.upcase }
    RUBY
    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end

  def test_proc_body_type_error
    source = <<~RUBY
      f = proc { 42.upcase }
    RUBY
    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end

  def test_proc_new_body_type_error
    source = <<~RUBY
      f = Proc.new { 42.upcase }
    RUBY
    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end
end
