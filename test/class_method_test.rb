# frozen_string_literal: true

require 'test_helper'

class ClassMethodTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # No Error
  # ============================================

  def test_class_method_basic_no_error
    source = <<~RUBY
      class User
        def self.create
          "created"
        end
      end

      User.create
    RUBY

    assert_no_check_errors(source)
  end

  def test_class_method_with_params_no_error
    source = <<~RUBY
      class User
        def self.find(id)
          "user"
        end
      end

      User.find(1)
    RUBY

    assert_no_check_errors(source)
  end

  def test_class_method_qualified_name_no_error
    source = <<~RUBY
      module Api
        class User
          def self.create
            "created"
          end
        end
      end

      Api::User.create
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_class_method_return_type_error
    source = <<~RUBY
      class User
        def self.create
          "created"
        end
      end

      User.create.even?
    RUBY

    assert_check_error(source, method_name: 'even?', receiver_type: 'String')
  end

  def test_class_method_chain_type_error
    source = <<~RUBY
      class User
        def self.count
          42
        end
      end

      User.count.upcase
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end

  def test_class_method_and_instance_method_coexist
    source = <<~RUBY
      class User
        def self.create
          "created"
        end

        def name
          "Alice"
        end
      end

      User.create.upcase
      User.new.name.upcase
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Reopened class / coexistence
  # ============================================

  def test_class_method_in_reopened_class_no_error
    source = <<~RUBY
      class User
        def self.create
          "created"
        end
      end

      class User
        def self.destroy
          "destroyed"
        end
      end

      User.create
      User.destroy
    RUBY

    assert_no_check_errors(source)
  end

  def test_class_method_return_type_used_correctly
    source = <<~RUBY
      class User
        def self.greeting
          "hello"
        end
      end

      User.greeting.upcase
    RUBY

    assert_no_check_errors(source)
  end
end
