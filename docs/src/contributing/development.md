# Development Setup

## Prerequisites

- Rust 1.70+
- Git

## Clone & Build

```bash
git clone https://github.com/rika-labs/repo-lint.git
cd repo-lint
cargo build
```

## Run Tests

```bash
cargo test
```

## Run with Coverage

```bash
cargo install cargo-llvm-cov
cargo llvm-cov --html
open target/llvm-cov/html/index.html
```

## Code Style

```bash
cargo fmt
cargo clippy -- -D warnings
```

## Project Structure

```
repo-lint/
├── src/
│   ├── main.rs           # CLI entry point
│   ├── lib.rs            # Library exports
│   ├── cli/              # Command implementations
│   ├── config/           # Config parsing (SWC)
│   ├── engine/           # Core matching logic
│   ├── output/           # Reporters (console, JSON, SARIF)
│   └── cache/            # Incremental caching
├── tests/
│   ├── integration/      # Integration tests
│   └── fixtures/         # Test fixtures
├── docs/                 # mdBook documentation
└── benches/              # Performance benchmarks
```

## Architecture

### Config Parsing

1. Read `repo-lint.config.ts`
2. Parse with SWC (TypeScript AST)
3. Evaluate restricted DSL functions
4. Emit ConfigIR (intermediate representation)

### Matching

1. Compile layout IR into path trie
2. Walk filesystem (parallel via `ignore` crate)
3. Match each path against trie
4. Evaluate rules (forbidPaths, forbidNames)
5. Collect violations

### Output

1. Sort violations by (path, rule_id)
2. Format via reporter (Console/JSON/SARIF)
3. Exit with appropriate code

## Pull Request Process

1. Fork the repository
2. Create feature branch
3. Write tests (target 80% coverage)
4. Run `cargo fmt` and `cargo clippy`
5. Submit PR with clear description
