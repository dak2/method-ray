$LOAD_PATH.unshift File.expand_path("../lib", __dir__)
require "methodray"

# Ensure RBS cache exists before any test runs (CheckTest CLI tests depend on it)
MethodRay.setup

require "minitest/autorun"
require "tempfile"
require "open3"

module CLITestHelper
  private

  def run_check(source)
    file = Tempfile.new(['integration_test', '.rb'])
    file.write(source)
    file.close

    stdout, stderr, status = Open3.capture3('bundle', 'exec', 'methodray', 'check', file.path)
    [stdout, stderr, status]
  ensure
    file&.unlink
  end

  def assert_check_error(source, method_name:, receiver_type:)
    stdout, _stderr, status = run_check(source)

    refute status.success?, "Expected check to fail but it succeeded"
    assert_match(/undefined method `#{Regexp.escape(method_name)}` for #{Regexp.escape(receiver_type)}/, stdout)
  end

  def assert_no_check_errors(source)
    stdout, _stderr, status = run_check(source)

    assert status.success?, "Expected check to pass but it failed.\nOutput: #{stdout}"
  end

  def assert_error_at(source, line:, column:)
    stdout, _stderr, status = run_check(source)

    refute status.success?, "Expected check to fail but it succeeded"
    assert_match(/:#{line}:#{column}:/, stdout, "Expected error at line #{line}, column #{column}")
  end
end
