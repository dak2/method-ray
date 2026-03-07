# MethodRay

A fast static type checker for Ruby. Verifies method call validity using graph-based type inference.

## Quick Reference

### Build & Test

```bash
# Rust tests (most frequently used)
cd rust && cargo test --lib

# Rust build (native extension)
bundle exec rake compile

# Ruby tests (integration tests)
bundle exec rake test

# Full build (compile → test)
bundle exec rake

# CLI binary build
cd rust && cargo build --release --bin methodray --features cli

# Clippy (lint)
cd rust && cargo clippy --lib --all-features -- -W clippy::all

# Ruby lint
bundle exec rubocop
```

### Running Individual Tests

```bash
# Rust: specific test
cd rust && cargo test --lib test_name

# Ruby: specific file
bundle exec ruby -Ilib -Itest test/string_test.rb
```

## Project Structure

```
rust/src/           # Rust core (type inference engine)
  analyzer/         #   AST → Graph conversion (install.rs is the entry point)
  env/              #   Environment (GlobalEnv, LocalEnv, MethodRegistry, Scope)
  graph/            #   Type graph (Vertex, Box, ChangeSet)
  rbs/              #   RBS loader & type conversion
  cache/            #   RBS binary cache
  types.rs          #   Type definitions (Instance, Generic, Union, Nil, Bot, Singleton)
  checker.rs        #   FileChecker (per-file type checking)
  parser.rs         #   ruby-prism + bumpalo parser
  cli/              #   CLI (check, watch, clear-cache)
  lsp/              #   LSP server
ext/                # Ruby FFI bridge (via magnus)
lib/                # Ruby library (gem entry point)
test/               # Ruby integration tests (minitest)
```

## Tech Stack

- **Rust**: Core engine (ruby-prism parser, graph-based type inference)
- **Ruby**: Gem interface, RBS loader, integration tests
- **FFI**: magnus crate (Rust <-> Ruby)
- **Build**: rb_sys + rake-compiler (cross-compilation support)

## Coding Conventions

### Rust

- Add new AST node handlers as separate files under `rust/src/analyzer/` (e.g., `loops.rs`, `exceptions.rs`)
- Follow the existing function signature pattern: `process_*_node(genv, lenv, changes, source, node) -> Option<VertexId>`
- When adding new nodes to `install.rs` dispatch, maintain the ordering: definitions → control flow → literals
- `#[allow(dead_code)]` is prohibited in principle — remove unused code instead
- Write tests in `#[cfg(test)] mod tests` within each module

### Ruby

- Always include `frozen_string_literal: true` in every file
- Follow RuboCop config (`.rubocop.yml`)
- Write tests with minitest under `test/` as `*_test.rb`

### Commit Messages

- Write in English
- Start with an imperative verb (e.g., `Add while/until loop support`, `Fix false positive on ...`)

## Architecture Overview

### Type Inference Pipeline

```
Ruby Source → ruby-prism AST → AstInstaller (Graph construction) → Constraint propagation → Type error detection
```

### Key Design Patterns

- **Vertex Graph**: Lazy type tracking with Source (fixed types) and Vertex (dynamic types)
- **Reactive Boxes**: Re-execute method call constraints when receiver type changes (MethodCallBox)
- **Merge Vertex**: Merge multiple return paths into a union type
- **Change Batching**: Batch edge updates via ChangeSet for atomic application
