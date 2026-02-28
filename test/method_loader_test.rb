# frozen_string_literal: true

require 'test_helper'

class MethodLoaderTest < Minitest::Test
  def setup
    require_relative '../rust/src/rbs/method_loader'
    @loader = Rbs::MethodLoader.new
    @results = @loader.load_methods
  end

  def test_existing_classes_still_loaded
    loaded_classes = @results.map { |r| r[:receiver_class] }.uniq
    %w[String Integer Float Array Hash Symbol Kernel Object].each do |cls|
      assert_includes loaded_classes, cls, "#{cls} should be loaded"
    end
  end
end
