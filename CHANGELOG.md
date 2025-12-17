# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2024-12-17

### Added

- Initial release of repo-lint
- TypeScript DSL config parser using SWC (no Node.js required)
- Layout enforcement with `dir`, `file`, `opt`, `param`, and `many` functions
- Case style validation (kebab, snake, camel, pascal)
- Rules engine with `forbidPaths` and `forbidNames`
- Three output formats: Console (colored), JSON, SARIF 2.1.0
- CLI commands: `check`, `scaffold`, `inspect`
- Parallel file walking using the `ignore` crate
- Streaming processing for large repositories

### Performance

- **~950k files/second** throughput
- 500k files processed in ~0.5 seconds
- Optimizations:
  - `fast-glob` for zero-allocation glob matching
  - `compact_str` for inline string storage
  - `crossbeam-channel` for lock-free parallelism
  - Early exit paths for common cases
