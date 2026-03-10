# frozen_string_literal: true

require 'bundler/gem_tasks'
require 'rb_sys/extensiontask'

GEMSPEC = Gem::Specification.load('methodray.gemspec')

# Native extension task (Ruby FFI via magnus)
RbSys::ExtensionTask.new('methodray', GEMSPEC) do |ext|
  ext.lib_dir = 'lib/methodray'
  ext.cross_compile = true
  ext.cross_platform = %w[
    x86_64-linux
    x86_64-linux-musl
    aarch64-linux
    x86_64-darwin
    arm64-darwin
    x64-mingw-ucrt
  ]
end

# CLI binary build task
namespace :cli do
  desc 'Build CLI binary for current platform'
  task :build do
    sh 'cd core && cargo build --release --bin methodray --features cli'
  end

  desc 'Build CLI binary for all platforms (requires cross)'
  task :cross do
    platforms = {
      'x86_64-unknown-linux-gnu' => 'x86_64-linux',
      'aarch64-unknown-linux-gnu' => 'aarch64-linux',
      'x86_64-apple-darwin' => 'x86_64-darwin',
      'aarch64-apple-darwin' => 'arm64-darwin'
    }

    platforms.each do |rust_target, ruby_platform|
      puts "Building for #{rust_target}..."
      sh "cd core && cross build --release --bin methodray --features cli --target #{rust_target}"

      # Copy binary to platform-specific directory
      binary_name = rust_target.include?('windows') ? 'methodray.exe' : 'methodray'
      src = "core/target/#{rust_target}/release/#{binary_name}"
      dst_dir = "lib/methodray/#{ruby_platform}"

      mkdir_p dst_dir
      cp src, "#{dst_dir}/#{binary_name}"
    end
  end
end

# Test task
require 'minitest/test_task'
Minitest::TestTask.create(:test) do |t|
  t.libs << 'test'
  t.libs << 'lib'
  t.test_globs = ['test/**/*_test.rb']
end

# Default task
task default: %i[compile test]
