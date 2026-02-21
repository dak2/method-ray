# frozen_string_literal: true

require_relative 'commands'

module MethodRay
  class CLI
    def self.start(args)
      command = args.shift

      case command
      when 'help', '--help', '-h', nil
        Commands.help
      when 'version', '--version', '-v'
        Commands.version
      when 'check'
        Commands.check(args)
      when 'watch'
        Commands.watch(args)
      when 'clear-cache'
        warn "WARNING: 'clear-cache' is deprecated and will be removed in a future version."
        Commands.clear_cache(args)
      else
        puts "Unknown command: #{command}"
        Commands.help
        exit 1
      end
    end
  end
end
