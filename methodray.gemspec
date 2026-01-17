require_relative "lib/methodray/version"

Gem::Specification.new do |spec|
  spec.name          = "method-ray"
  spec.version       = MethodRay::VERSION
  spec.authors       = ["dak2"]
  spec.email         = [""]

  spec.summary       = "Method-Ray is a fast static analysis tool for Ruby methods."
  spec.description   = <<~EOD
    Method-Ray is a static analysis tool that checks the callability of methods in Ruby code.
  EOD
  spec.homepage      = "https://github.com/dak2/method-ray"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4"

  spec.files = Dir[
    "lib/**/*.rb",
    "exe/*",
    "ext/**/*",
    "src/**/*.rs",
    "src/**/*.rb",
    "Cargo.toml",
    "Cargo.lock",
    "README.md"
  ]

  spec.bindir        = "exe"
  spec.executables   = ["methodray"]
  spec.require_paths = ["lib"]
  spec.extensions    = ["ext/extconf.rb"]

  # Runtime dependencies
  spec.add_dependency "rbs", "~> 3.0"

  # Development dependencies
  spec.add_development_dependency "rake", "~> 13.0"
  spec.add_development_dependency "rb_sys", "~> 0.9"
  spec.add_development_dependency "rake-compiler", "~> 1.2"
  spec.add_development_dependency "minitest", "~> 5.0"
end
