# frozen_string_literal: true

require 'test_helper'

class ConstantReadTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # No Error
  # ============================================

  def test_user_new_method_call
    source = <<~RUBY
      class User
        def name
          "Alice"
        end
      end

      User.new.name.upcase
    RUBY

    assert_no_check_errors(source)
  end

  def test_user_new_assign_then_call
    source = <<~RUBY
      class User
        def name
          "Alice"
        end
      end

      user = User.new
      user.name.upcase
    RUBY

    assert_no_check_errors(source)
  end

  def test_user_new_with_attr_reader
    source = <<~RUBY
      class User
        attr_reader :name

        def initialize
          @name = "Alice"
        end
      end

      user = User.new
      user.name.upcase
    RUBY

    assert_no_check_errors(source)
  end

  def test_user_new_with_initialize_and_ivar
    source = <<~RUBY
      class User
        attr_reader :name

        def initialize(name)
          @name = name
        end
      end

      user = User.new("Alice")
      user.name.upcase
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Qualified name (ConstantPathNode)
  # ============================================

  def test_qualified_constant_path_new_method_call
    source = <<~RUBY
      module Api
        class User
          def name
            "Alice"
          end
        end
      end

      Api::User.new.name.upcase
    RUBY

    assert_no_check_errors(source)
  end

  def test_same_class_name_different_namespace
    source = <<~RUBY
      module Api
        class User
          def name
            "api_user"
          end
        end
      end

      module Admin
        class User
          def role
            "admin"
          end
        end
      end

      Api::User.new.name.upcase
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_user_new_method_chain_type_error
    source = <<~RUBY
      class User
        def name
          "Alice"
        end
      end

      User.new.name.even?
    RUBY

    assert_check_error(source, method_name: 'even?', receiver_type: 'String')
  end

  def test_user_new_with_attr_type_error
    source = <<~RUBY
      class User
        attr_reader :age

        def initialize
          @age = 30
        end
      end

      User.new.age.upcase
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end

  def test_qualified_constant_path_new_type_error
    source = <<~RUBY
      module Api
        class User
          def name
            "Alice"
          end
        end
      end

      Api::User.new.name.even?
    RUBY

    assert_check_error(source, method_name: 'even?', receiver_type: 'String')
  end
end
