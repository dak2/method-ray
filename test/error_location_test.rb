# frozen_string_literal: true

require 'test_helper'

class ErrorLocationTest < Minitest::Test
  include CLITestHelper

  # ============================================
  # Error Location
  # ============================================

  def test_error_location_points_to_dot
    source = "name = \"x\"\nname.abs"

    assert_error_at(source, line: 2, column: 5)
  end

  def test_error_location_method_chain
    source = "x = \"hello\"\ny = x.upcase.foo"

    assert_error_at(source, line: 2, column: 13)
  end
end
