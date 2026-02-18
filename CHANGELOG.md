# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.1.4]: https://github.com/dak2/method-ray/releases/tag/v0.1.4
[0.1.3]: https://github.com/dak2/method-ray/releases/tag/v0.1.3
[0.1.2]: https://github.com/dak2/method-ray/releases/tag/v0.1.2
[0.1.1]: https://github.com/dak2/method-ray/releases/tag/v0.1.1
[0.1.0]: https://github.com/dak2/method-ray/releases/tag/v0.1.0
