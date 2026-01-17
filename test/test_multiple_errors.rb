x = 1
y = x.upcase  # Error: Integer#upcase

a = "hello"
b = a.unknown_method  # Error: String#unknown_method

c = 42
d = c.foo  # Error: Integer#foo
