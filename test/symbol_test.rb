# frozen_string_literal: true

require 'test_helper'

class SymbolTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # Error Detection
  # ============================================

  def test_interpolated_symbol_type_error
    source = <<~RUBY
      class Formatter
        def format
          x = :"hello_\#{1}"
          y = x.ceil
        end
      end
    RUBY
    assert_check_error(source, method_name: 'ceil', receiver_type: 'Symbol')
  end
end
