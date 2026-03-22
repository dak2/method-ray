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

### Checking Methods

```bash
# Check a single file
bundle exec methodray check app/models/user.rb
```

### Watching for File Changes, Re-checking Methods

```bash
# Watch a file for changes and re-check on save
bundle exec methodray watch app/models/user.rb
```

#### Example Usage

`bundle exec methodray check app/models/user.rb`

```ruby
# app/models/user.rb
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
