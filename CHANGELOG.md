# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.10] - 2026-03-24

### Added

- Add safe navigation operator (`&.`) support ([#65](https://github.com/dak2/method-ray/pull/65))
- Add inheritance chain method resolution for user-defined classes ([#66](https://github.com/dak2/method-ray/pull/66))
- Add extend support for module methods as class methods ([#67](https://github.com/dak2/method-ray/pull/67))
- Add pull request template ([#82](https://github.com/dak2/method-ray/pull/82))

### Changed

- Migrate Rust integration tests to Ruby integration tests ([#68](https://github.com/dak2/method-ray/pull/68), [#69](https://github.com/dak2/method-ray/pull/69), [#70](https://github.com/dak2/method-ray/pull/70), [#71](https://github.com/dak2/method-ray/pull/71), [#72](https://github.com/dak2/method-ray/pull/72), [#73](https://github.com/dak2/method-ray/pull/73), [#74](https://github.com/dak2/method-ray/pull/74), [#75](https://github.com/dak2/method-ray/pull/75))
- Remove redundant Rust unit tests ([#77](https://github.com/dak2/method-ray/pull/77), [#78](https://github.com/dak2/method-ray/pull/78), [#80](https://github.com/dak2/method-ray/pull/80), [#83](https://github.com/dak2/method-ray/pull/83), [#84](https://github.com/dak2/method-ray/pull/84), [#85](https://github.com/dak2/method-ray/pull/85), [#86](https://github.com/dak2/method-ray/pull/86))
- Simplify README to focus on core value proposition ([#76](https://github.com/dak2/method-ray/pull/76))

### Fixed

- Fix release workflow to include gem assets in GitHub Release ([#63](https://github.com/dak2/method-ray/pull/63), [#64](https://github.com/dak2/method-ray/pull/64))

## [0.1.9] - 2026-03-15

### Added

- Add linter CI workflow for Clippy and RuboCop ([#61](https://github.com/dak2/method-ray/pull/61))
- Add unit tests for blocks and parameters analyzers ([#60](https://github.com/dak2/method-ray/pull/60))
- Add super call type inference support ([#59](https://github.com/dak2/method-ray/pull/59))
- Add for loop type inference support ([#58](https://github.com/dak2/method-ray/pull/58))
- Add module include support for mixin method resolution ([#57](https://github.com/dak2/method-ray/pull/57))
- Add complete multi-assignment type inference ([#56](https://github.com/dak2/method-ray/pull/56))
- Add auto-tagging workflow for release PR merges ([#53](https://github.com/dak2/method-ray/pull/53))

### Changed

- Rename rust/ directory to core/ ([#55](https://github.com/dak2/method-ray/pull/55))
- Infer actual exception types in rescue clauses ([#54](https://github.com/dak2/method-ray/pull/54))

## [0.1.8] - 2026-03-09

### Added

- while/until loop support to type inference ([#46](https://github.com/dak2/method-ray/pull/46))
- Not operator (!) support to type inference ([#47](https://github.com/dak2/method-ray/pull/47))
- begin/rescue/ensure exception handling support to type inference ([#48](https://github.com/dak2/method-ray/pull/48))
- Keyword argument support to type inference ([#49](https://github.com/dak2/method-ray/pull/49))
- Multiple assignment support to type inference ([#50](https://github.com/dak2/method-ray/pull/50))

### Changed

- Resolve all Clippy warnings for cleaner, more idiomatic Rust ([#51](https://github.com/dak2/method-ray/pull/51))

## [0.1.7] - 2026-03-07

### Added

- Kernel/Object methods loaded from RBS to reduce false positives ([#39](https://github.com/dak2/method-ray/pull/39))
- Object/Kernel fallback chain for method resolution ([#40](https://github.com/dak2/method-ray/pull/40))
- Constant namespace resolution for ConstantReadNode in nested scopes ([#41](https://github.com/dak2/method-ray/pull/41))
- Cargo test added to CI workflow ([#38](https://github.com/dak2/method-ray/pull/38))

### Changed

- Extract `bytes_to_name` helper to consolidate 17 UTF-8 conversion sites ([#42](https://github.com/dak2/method-ray/pull/42))
- Refactor MethodCallBox by extracting helper methods ([#43](https://github.com/dak2/method-ray/pull/43))

## [0.1.6] - 2026-02-23

### Fixed

- Embed `method_loader.rb` at compile time with `include_str!` to eliminate runtime dependency on source directory ([#35](https://github.com/dak2/method-ray/pull/35))

## [0.1.5] - 2026-02-23

### Added

- String interpolation type inference (`InterpolatedStringNode`, `InterpolatedSymbolNode`, `InterpolatedRegularExpressionNode`) ([#26](https://github.com/dak2/method-ray/pull/26))
- Parentheses node type inference for parenthesized expressions ([#27](https://github.com/dak2/method-ray/pull/27))
- Qualified name method registration to resolve namespace conflicts ([#28](https://github.com/dak2/method-ray/pull/28))
- Return statement type inference with merge vertex pattern ([#29](https://github.com/dak2/method-ray/pull/29))
- Ternary operator type inference tests ([#30](https://github.com/dak2/method-ray/pull/30))
- Logical operator (`&&`/`||`) type inference with union type approximation ([#31](https://github.com/dak2/method-ray/pull/31))
- Class method (`def self.foo`) type registration and checking ([#32](https://github.com/dak2/method-ray/pull/32))

### Changed

- Removed stateless `Analyzer` class and simplified Ruby FFI surface to module functions ([#33](https://github.com/dak2/method-ray/pull/33))

### Deprecated

- `clear_cache` command ([#25](https://github.com/dak2/method-ray/pull/25))

## [0.1.4] - 2026-02-16

### Added

- Method return type inference for user-defined methods ([#18](https://github.com/dak2/method-ray/pull/18))
- Parameter type propagation from call-site arguments to method parameters ([#19](https://github.com/dak2/method-ray/pull/19))
- Receiver-less method call support (ImplicitSelfCall) ([#20](https://github.com/dak2/method-ray/pull/20))
- `attr_reader`/`attr_writer`/`attr_accessor` support for type inference ([#21](https://github.com/dak2/method-ray/pull/21))
- `if`/`unless`/`case` conditional type inference ([#22](https://github.com/dak2/method-ray/pull/22))
- `ConstantReadNode`/`ConstantPathNode` support for type inference ([#23](https://github.com/dak2/method-ray/pull/23))

### Changed

- Split install.rs and integration tests into focused modules ([#17](https://github.com/dak2/method-ray/pull/17))

## [0.1.3] - 2025-02-08

### Added

- Method parameter type inference support ([#3](https://github.com/dak2/method-ray/pull/3))
- Block parameter type variable resolution ([#4](https://github.com/dak2/method-ray/pull/4))
- Module scope support ([#6](https://github.com/dak2/method-ray/pull/6))
- Fully qualified name support for nested classes/modules ([#7](https://github.com/dak2/method-ray/pull/7))
- Float type support ([#8](https://github.com/dak2/method-ray/pull/8))
- Regexp type support ([#9](https://github.com/dak2/method-ray/pull/9))
- Range type support ([#10](https://github.com/dak2/method-ray/pull/10))
- Generic type inference for Range, Hash, and nested Array ([#11](https://github.com/dak2/method-ray/pull/11))

### Fixed

- Call operator location ([#12](https://github.com/dak2/method-ray/pull/12))
- Memory leak ([#13](https://github.com/dak2/method-ray/pull/13))

### Changed

- Extract BinaryLocator class from Commands module ([#5](https://github.com/dak2/method-ray/pull/5))
- Migrate Rust integration tests to Ruby CLI and Rust unit tests ([#14](https://github.com/dak2/method-ray/pull/14))
- Remove unnecessary test files and logs ([#1](https://github.com/dak2/method-ray/pull/1), [#15](https://github.com/dak2/method-ray/pull/15))

## [0.1.2] - 2025-01-19

### Added

- Pre-built RBS cache bundled with gem (no initialization required)
- `MethodRay.setup` for cache generation (internal API)

### Changed

- Separated `setup` logic from `infer_types` for cleaner cache generation

## [0.1.1] - 2025-01-19

### Added

- aarch64-linux (ARM64 Linux) support
- macOS support (arm64-darwin)

## [0.1.0] - 2025-01-18

### Added

- Initial release
- `methodray check` - Static type checking for Ruby files

[0.1.10]: https://github.com/dak2/method-ray/releases/tag/v0.1.10
[0.1.9]: https://github.com/dak2/method-ray/releases/tag/v0.1.9
[0.1.8]: https://github.com/dak2/method-ray/releases/tag/v0.1.8
[0.1.7]: https://github.com/dak2/method-ray/releases/tag/v0.1.7
[0.1.6]: https://github.com/dak2/method-ray/releases/tag/v0.1.6
[0.1.5]: https://github.com/dak2/method-ray/releases/tag/v0.1.5
[0.1.4]: https://github.com/dak2/method-ray/releases/tag/v0.1.4
[0.1.3]: https://github.com/dak2/method-ray/releases/tag/v0.1.3
[0.1.2]: https://github.com/dak2/method-ray/releases/tag/v0.1.2
[0.1.1]: https://github.com/dak2/method-ray/releases/tag/v0.1.1
[0.1.0]: https://github.com/dak2/method-ray/releases/tag/v0.1.0
