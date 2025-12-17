# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2024-12-17

### Added

- **recursive()** - Match arbitrary-depth nested structures (perfect for Next.js App Router)
  ```typescript
  $routes: recursive(param({ case: 'kebab' }, dir({ 'page.tsx': file() })))
  ```
- **either()** - Match file OR directory at same position
  ```typescript
  $segment: either(file('page.tsx'), dir({ 'index.ts': file() }))
  ```
- **ignore** config option - Ignore specific directories
  ```typescript
  ignore: ['.git', 'node_modules', '.next']
  ```
- **useGitignore** config option - Honor .gitignore files (default: true)
- **ignorePaths** rule - Skip paths entirely (no violations, unlike forbidPaths)
  ```typescript
  rules: { ignorePaths: ['**/node_modules/**', '**/.turbo/**'] }
  ```
- **--scope** CLI flag - Validate only a subtree
  ```bash
  repo-lint check --scope apps/sentinel
  ```
- **Better error messages** - Match attempts shown when path doesn't match layout
- **Next.js preset** - Ready-to-use App Router layout
  ```typescript
  import { nextjsAppRouter } from '@rikalabs/repo-lint/presets'
  layout: nextjsAppRouter({ routeCase: 'kebab' })
  ```

### Fixed

- .gitignore now honored by default (fixes 6000+ false positives from .git/)

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
