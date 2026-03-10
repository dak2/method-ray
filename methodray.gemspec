# frozen_string_literal: true

require_relative 'lib/methodray/version'

Gem::Specification.new do |spec|
  spec.name          = 'method-ray'
  spec.version       = MethodRay::VERSION
  spec.authors       = ['dak2']
  spec.email         = ['dak2.dev@gmail.com']

  spec.summary       = 'Method-Ray is a fast static analysis tool for Ruby methods.'
  spec.description   = <<~DESCRIPTION
    Method-Ray is a static analysis tool that checks the callability of methods in Ruby code.
    It uses graph-based type inference to detect undefined method calls at analysis time.
  DESCRIPTION
  spec.homepage      = 'https://github.com/dak2/method-ray'
  spec.license       = 'MIT'
  spec.required_ruby_version = '>= 3.4.0'

  spec.metadata = {
    'homepage_uri' => spec.homepage,
    'source_code_uri' => spec.homepage,
    'changelog_uri' => "#{spec.homepage}/blob/main/CHANGELOG.md",
    'bug_tracker_uri' => "#{spec.homepage}/issues",
    'rubygems_mfa_required' => 'true'
  }

  spec.files = Dir[
    'lib/**/*.rb',
    'lib/methodray/*.{so,dylib,dll,bundle}', # Precompiled native extensions
    'exe/*',
    'ext/**/*',
    'core/**/*.rs',
    'core/Cargo.toml',
    'core/Cargo.lock',
    'README.md',
    'LICENSE',
    'CHANGELOG.md'
  ]

  spec.bindir        = 'exe'
  spec.executables   = ['methodray']
  spec.require_paths = ['lib']
  spec.extensions    = ['ext/extconf.rb']

  # Runtime dependencies
  spec.add_dependency 'rbs', '~> 3.0'
end
