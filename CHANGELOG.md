# Changelog

All notable changes to this project will be documented in this file.

## [0.3.11] - 2024-12-31

### Added

- **Import Arbitrary Constants**: Config files can now import and use string arrays, rules configs, and mirror configs from other modules. Previously, only `layout` and `when` could be imported.
  ```typescript
  import { elysiaDefaultIgnore, elysiaDefaultRules, elysiaMirrorConfig } from '@/shared/elysia'

  export default defineConfig({
    ignore: elysiaDefaultIgnore,  // Now works!
    rules: elysiaDefaultRules,    // Now works!
    mirror: elysiaMirrorConfig,   // Now works!
  })
  ```

### Fixed

- **Mirror Path Resolution**: Fixed mirror target path computation for patterns with multiple wildcards. Previously, patterns like `src/modules/*/*.ts -> tests/modules/*/*.test.ts` incorrectly computed the target path. Now wildcards are properly extracted from the source and substituted into the target pattern.

## [0.3.10] - 2024-12-19

### Fixed

- **Bun Compatibility**: Added lazy install fallback for bun users. Bun doesn't run postinstall scripts by default, so the CLI wrapper now automatically downloads the binary on first run if missing.
- **CLI Wrapper**: Renamed `bin/repo-lint` to `bin/repo-lint.js` for better cross-platform compatibility.
- **Skip Install Option**: Added `REPO_LINT_SKIP_INSTALL` environment variable to opt out of automatic binary download.

## [0.3.9] - 2024-12-17

### Fixed

- **Brace Expansion in File Patterns**: Fixed glob matching to properly support brace expansion patterns like `*.{ts,tsx}`. Previously, patterns with braces were treated as literal strings instead of glob alternations.
- **Complex Glob Patterns**: Replaced simple pattern matching with `fast_glob` for full glob support including character classes, nested braces, and other glob features.

## [0.3.8] - 2024-12-17

### Fixed

- **Re-exported Layouts**: Fixed support for `export { layout } from './module'` syntax. Layouts re-exported from intermediate modules are now properly resolved.
- **Local Const Resolution in Imports**: When importing a layout that references local const variables, those consts are now correctly resolved within the imported file's scope.
- **Named Export Support**: Added support for `export { localVar }` syntax (local variable re-exports) in addition to `export const`.

## [0.3.7] - 2024-12-17

### Fixed

- **Expected Children Traversal**: Fixed `get_expected_children` to correctly traverse param nodes at directory level.

## [0.3.6] - 2024-12-17

### Fixed

- **Object Shorthand Support**: Fixed parsing of object shorthand syntax in directory children (e.g., `directory({ layout })` where `layout` is a variable).

## [0.3.5] - 2024-12-17

### Fixed

- **Imported Layouts with When**: Fixed support for importing `layout` and `when` configurations from other modules.

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
