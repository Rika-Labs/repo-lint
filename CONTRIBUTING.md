# Contributing to repo-lint

Thank you for your interest in contributing to repo-lint!

## Development Setup

### Prerequisites

- Rust 1.70+
- Git

### Clone and Build

```bash
git clone https://github.com/rika-labs/repo-lint.git
cd repo-lint
cargo build
```

### Run Tests

```bash
cargo test
```

### Run Benchmarks

```bash
cargo run --release --example benchmark
```

### Code Style

```bash
cargo fmt
cargo clippy -- -D warnings
```

## Pull Request Process

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Write tests for your changes
4. Ensure all tests pass: `cargo test`
5. Ensure code is formatted: `cargo fmt`
6. Ensure no clippy warnings: `cargo clippy -- -D warnings`
7. Commit your changes with a clear message
8. Push and create a Pull Request

## Code Coverage

We target 80% code coverage. Run coverage locally:

```bash
cargo install cargo-llvm-cov
cargo llvm-cov --html
open target/llvm-cov/html/index.html
```

## Release Process

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create a git tag: `git tag v0.x.y`
4. Push the tag: `git push origin v0.x.y`
5. GitHub Actions will automatically:
   - Build binaries for all platforms
   - Create a GitHub Release
   - Publish to crates.io

## Questions?

Open an issue or start a discussion on GitHub.
