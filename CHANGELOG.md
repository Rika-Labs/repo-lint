# Changelog

All notable changes to this project will be documented in this file.

## [0.3.4] - 2024-12-17

### Fixed

- **Quality Gate Fixes**: Resolved clippy warnings and formatting issues in the config parser.
- **Publish Fix**: Bumped version to retry failed npm release.

## [0.3.3] - 2024-12-17

### Added

- **Monorepo Support & Config Inheritance**
  - **Config Inheritance (`extends`)**: Configurations can now extend other config files, allowing for shared base rules and layouts across a monorepo.
  - **Workspace Import Resolution**: Support for ES module `import` statements in configuration files. Resolves relative paths and `node_modules` without needing a local Node.js environment.
  - **Built-in Presets**: New `nextjsPreset()` function for quickly setting up Next.js App Router validation.
  - **Optional Layout**: Root configurations can now skip the `layout` property, making them perfect for pure workspace discovery and global rule definitions.

### Fixed

- Removed flaky test caused by HashMap iteration order non-determinism
- Fixed cross-platform path handling in recursive pattern tests

### Changed

- Release workflow now requires quality gate (fmt, clippy, tests on all platforms) before build

## [0.3.2] - 2024-12-17

### Fixed

- **Bug #1: `ignore` config now works for directories** - Previously `ignore: ['apps', 'packages']` didn't work; files inside were still checked. Now directories and glob patterns like `**/utils/**` are properly ignored during file walking.
- **Bug #2: `inspect path` and `check` now consistent** - `inspect path` now shows violations from rules (forbidPaths, ignorePaths) and reports if a path is ignored, matching `check` behavior.
- **Better recursive pattern error messages** - When recursion depth is exceeded, error now shows the level and path where it failed, plus a hint to increase maxDepth.

### Added

- **Documentation for strict mode behavior** - Clarified that `strict: true` rejects files not matching any pattern in the directory.
- **Documentation for file pattern + case validation** - Clarified that case validation only applies to files matching the pattern.

## [0.3.1] - 2024-12-17

### Fixed

- Fixed Windows path separator handling in config discovery (backslash → forward slash)
- Fixed cargo fmt issues in post_validator.rs

## [0.3.0] - 2024-12-17

### Added

- **API Improvements** - Clearer function names with backward compatibility
  - `directory()` - alias for `dir()` (both still work)
  - `optional()` - alias for `opt()` (both still work)
  - `required()` - mark files/directories that must exist

- **Strict Mode** - Reject files not explicitly defined in layout
  ```typescript
  directory({ ... }, { strict: true })
  ```

- **File Case Validation** - Enforce naming conventions on files
  ```typescript
  $files: many(file({ pattern: "*.ts", case: "kebab" }))
  ```

- **Depth Limits** - Control maximum directory nesting
  ```typescript
  directory({ ... }, { maxDepth: 3 })
  ```

- **Count Limits** - Limit number of files matching `many()`
  ```typescript
  $files: many({ max: 10 }, file("*.ts"))
  ```

- **Dependencies Validation** - Require related files exist
  ```typescript
  dependencies: {
    "src/controllers/*.ts": "src/services/*.ts"
  }
  ```

- **Mirror Validation** - Enforce structural mirroring
  ```typescript
  mirror: [{
    source: "src/components/*",
    target: "src/components/*.test.tsx",
    pattern: "*.tsx -> *.test.tsx"
  }]
  ```

- **When Conditions** - Conditional file requirements
  ```typescript
  when: {
    "controller.ts": { requires: ["service.ts"] }
  }
  ```

- **Sub-path Imports** - Cleaner imports for specific functions
  ```typescript
  import { directory } from "@rikalabs/repo-lint/directory";
  import { optional } from "@rikalabs/repo-lint/optional";
  import { required } from "@rikalabs/repo-lint/required";
  import { file } from "@rikalabs/repo-lint/file";
  ```

### Changed

- PostValidator now runs after file walk for required/dependencies/mirror/when validations
- Depth tracking throughout layout matching for maxDepth enforcement

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
