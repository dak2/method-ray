# frozen_string_literal: true

require_relative 'binary_locator'

module MethodRay
  module Commands
    class << self
      def help
        puts <<~HELP
          MethodRay v#{MethodRay::VERSION} - A fast static analysis tool for Ruby methods.

          Usage:
            methodray help                    # Show this help
            methodray version                 # Show version
            methodray check [FILE] [OPTIONS]  # Type check a Ruby file
            methodray watch FILE              # Watch file for changes and auto-check
            methodray clear-cache             # Clear RBS method cache

          Examples:
            methodray check app/models/user.rb
            methodray watch app/models/user.rb
        HELP
      end

      def version
        puts "MethodRay v#{MethodRay::VERSION}"
      end

      def check(args)
        exec_rust_cli('check', args)
      end

      def watch(args)
        exec_rust_cli('watch', args)
      end

      def clear_cache(args)
        exec_rust_cli('clear-cache', args)
      end

      private

      def exec_rust_cli(command, args)
        binary_path = BinaryLocator.new.find

        unless binary_path
          warn 'Error: CLI binary not found.'
          warn ''
          warn 'For development, build with:'
          warn '  cd rust && cargo build --release --bin methodray --features cli'
          warn ''
          warn 'If installed via gem, this might be a platform compatibility issue.'
          warn 'Please report at: https://github.com/dak2/method-ray/issues'
          exit 1
        end

        exec(binary_path, command, *args)
      end
    end
  end
end
