# frozen_string_literal: true

require 'test_helper'

class AttrTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # No Error
  # ============================================

  def test_attr_reader_with_self_call
    source = <<~RUBY
      class User
        attr_reader :name

        def initialize
          @name = "Alice"
        end

        def greet
          self.name.upcase
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_attr_reader_with_receiverless_call
    source = <<~RUBY
      class User
        attr_reader :name

        def initialize
          @name = "Alice"
        end

        def greet
          name.upcase
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_attr_accessor_getter
    source = <<~RUBY
      class User
        attr_accessor :age

        def initialize
          @age = 30
        end

        def check
          self.age.even?
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_attr_reader_multiple_attributes
    source = <<~RUBY
      class User
        attr_reader :name, :email

        def initialize
          @name = "Alice"
          @email = "alice@test.com"
        end

        def display
          self.name.upcase
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  def test_attr_reader_before_initialize
    source = <<~RUBY
      class User
        attr_reader :name

        def initialize
          @name = "Alice"
        end
      end
    RUBY

    assert_no_check_errors(source)
  end

  # ============================================
  # Error Detection
  # ============================================

  def test_attr_reader_getter_type_error
    source = <<~RUBY
      class User
        attr_reader :name

        def initialize
          @name = "Alice"
        end

        def greet
          self.name.even?
        end
      end
    RUBY

    assert_check_error(source, method_name: 'even?', receiver_type: 'String')
  end

  def test_attr_reader_receiverless_type_error
    source = <<~RUBY
      class User
        attr_reader :age

        def initialize
          @age = 30
        end

        def check
          age.upcase
        end
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end

  def test_attr_accessor_getter_type_error
    source = <<~RUBY
      class User
        attr_accessor :count

        def initialize
          @count = 10
        end

        def display
          self.count.upcase
        end
      end
    RUBY

    assert_check_error(source, method_name: 'upcase', receiver_type: 'Integer')
  end
end
