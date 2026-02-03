# Method-Ray

A fast static callable method checker for Ruby code.

No type annotations required, just check callable methods in your Ruby files.

## Requirements

Method-Ray supports Ruby 3.4 or later.

## Installation

```bash
gem install methodray
```

## Quick Start

### VSCode Extension (under development)

1. Install the [Method-Ray VSCode extension](https://github.com/dak2/method-ray-vscode)
2. Open a Ruby file in VSCode
3. Errors will be highlighted automatically

### CLI

```bash
# Check a single file
bundle exec methodray check app/models/user.rb

# Watch mode - auto re-check on file changes
bundle exec methodray watch app/models/user.rb
```

#### Example

`methodray check <file>`: Performs static type checking on the specified Ruby file.


```ruby
class User
  def greeting
    name = "Alice"
    message = name.abs
    message
  end
end
```

This will output:

```
$ bundle exec methodray check app/models/user.rb
app/models/user.rb:4:19: error: undefined method `abs` for String
       message = name.abs
                     ^
```

## Contributing

Bug reports and pull requests are welcome on GitHub at this repository!

## License

MIT License. See [LICENSE](https://github.com/dak2/method-ray/blob/main/LICENSE) file for details.
