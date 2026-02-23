# frozen_string_literal: true

require 'rbs'

# TODO: use ruby-rbs crate when available
# https://github.com/ruby/rbs/pull/2808
return if defined?(Rbs::MethodLoader)

module Rbs
  class MethodLoader
    TARGET_CLASSES = %w[
      String Integer Float Array Hash Symbol
      TrueClass FalseClass NilClass
      Range Regexp Struct Enumerable
    ].freeze

    def initialize
      loader = ::RBS::EnvironmentLoader.new
      env = ::RBS::Environment.from_loader(loader).resolve_type_names
      @builder = ::RBS::DefinitionBuilder.new(env: env)
    end

    def load_methods
      results = []

      TARGET_CLASSES.each do |class_name|
        type_name = ::RBS::TypeName.new(
          name: class_name.to_sym,
          namespace: ::RBS::Namespace.root
        )

        definition = @builder.build_instance(type_name)

        definition.methods.each do |method_name, method_def|
          # Find a method_type with block if available, otherwise use first
          method_type_with_block = method_def.method_types.find(&:block)
          method_type = method_type_with_block || method_def.method_types.first
          next unless method_type

          return_type = method_type.type.return_type.to_s
          block_param_types = extract_block_param_types(method_type)

          results << {
            receiver_class: class_name,
            method_name: method_name.to_s,
            return_type: return_type,
            block_param_types: block_param_types
          }
        end
      rescue StandardError => e
        warn "Skipped #{class_name}: #{e.message}" if ENV['DEBUG']
      end

      results
    end

    private

    # Extract block parameter types from method_type
    # Returns nil if no block, or array of type strings
    def extract_block_param_types(method_type)
      return nil unless method_type.block

      block_func = method_type.block.type
      return nil unless block_func.is_a?(::RBS::Types::Function)

      param_types = []

      # Required positional parameters
      block_func.required_positionals.each do |param|
        param_types << param.type.to_s
      end

      # Optional positional parameters
      block_func.optional_positionals.each do |param|
        param_types << param.type.to_s
      end

      param_types.empty? ? nil : param_types
    end
  end
end
