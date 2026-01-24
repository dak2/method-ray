# frozen_string_literal: true

module MethodRay
  class BinaryLocator
    LIB_DIR = __dir__

    def initialize
      @binary_name = Gem.win_platform? ? 'methodray-cli.exe' : 'methodray-cli'
      @legacy_binary_name = Gem.win_platform? ? 'methodray.exe' : 'methodray'
    end

    def find
      candidates.find { |path| File.executable?(path) }
    end

    private

    def candidates
      [
        # CLI binary built during gem install (lib/methodray directory)
        File.expand_path(@binary_name, LIB_DIR),
        # Development: target/release (project root)
        File.expand_path("../../target/release/#{@binary_name}", LIB_DIR),
        # Development: rust/target/release (legacy standalone binary)
        File.expand_path("../../rust/target/release/#{@legacy_binary_name}", LIB_DIR)
      ]
    end
  end
end
