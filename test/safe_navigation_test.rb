# frozen_string_literal: true

require 'test_helper'

class SafeNavigationTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # No Error
  # ============================================

  def test_safe_navigation_basic_no_error
    source = <<~RUBY
      class User
        def name
          "Alice"
        end
      end

      User.new&.name
    RUBY

    assert_no_check_errors(source)
  end

  def test_safe_navigation_nil_receiver_no_error
    source = <<~RUBY
      x = nil
      x&.foo
    RUBY

    assert_no_check_errors(source)
  end

  def test_safe_navigation_chain_no_error
    source = <<~RUBY
      class Profile
        def name
          "Alice"
        end
      end

      class User
        def profile
          Profile.new
        end
      end

      User.new&.profile&.name
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_safe_navigation_undefined_method
    source = <<~RUBY
      class User
        def name
          "Alice"
        end
      end

      User.new&.undefined_method
    RUBY

    assert_check_error(source, method_name: 'undefined_method', receiver_type: 'User')
  end
end
