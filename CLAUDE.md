# repo-lint - Project Guidelines

**Repository:** [Rika-Labs/repo-lint](https://github.com/Rika-Labs/repo-lint)

## Overview
Extremely fast repository architecture linter with YAML configuration.

## Tech Stack
- **Runtime**: Bun
- **Core**: Effect TS, @effect/schema
- **Linting**: Oxlint
- **Git Hooks**: Husky + lint-staged

## Commands
- `bun run dev -- <args>` - Run CLI in development
- `bun run lint` - Lint with Oxlint
- `bun test` - Run tests
- `bun test --coverage` - Run tests with coverage (target: 90%+)
- `bun run build` - Build for production

## CLI Usage

```bash
# Lint current directory
repo-lint check

# With options
repo-lint check --config .repo-lint.yaml --json
repo-lint check --sarif  # GitHub Code Scanning format
repo-lint check --no-cache
repo-lint check --max-depth 3 --timeout-ms 10000 --no-gitignore

# Inspect config
repo-lint inspect layout
repo-lint inspect rule <rule-name>
```

## Code Standards

### Effect TS Patterns
- Use `Effect.gen` for generator-based workflows
- Use `Effect.tryPromise` for external async calls with typed errors
- Use `Effect.forEach` with `{ concurrency: N }` for bounded parallel operations (N = 10 default)
- Errors should extend `Data.TaggedError` with `_tag` discriminator
- Prefer `Effect.orElseSucceed`, `Effect.catchAll` for error recovery
- Use `Option` for optional values, never `undefined | T`
- Use `Ref` for mutable state in Effect context

### TypeScript
- Strict mode enabled
- `noUncheckedIndexedAccess: true`
- All tests must be type-safe (no `any`)
- Use discriminated unions for error types
- Use @effect/schema for runtime validation

### Testing
- All mocks must be properly typed
- Use `Effect.runPromise` / `Effect.runPromiseExit` for testing Effects
- Use `Cause.failureOption` for type-safe error assertions
- Target 90%+ line coverage

### Oxlint
- Strict config in `oxlint.json`
- No `var`, prefer `const`
- Use template literals
- Arrow functions preferred
- No nested ternaries

## File Structure
```
src/
  cli/              # CLI entry point and argument parser
    index.ts        # Main CLI entry
    parser.ts       # Argument parsing with Option types
  commands/         # Command handlers
    check.ts        # Check command implementation
    inspect.ts      # Inspect command implementation
  config/           # Configuration loading & validation
    loader.ts       # Config loading with circular extends detection
    presets/        # Built-in presets (nextjs, etc.)
  core/             # Core utilities
    case.ts         # Case style validation (kebab, snake, camel, pascal)
    matcher.ts      # Glob pattern matching with picomatch
    scanner.ts      # File system scanning with symlink/depth protection
  rules/            # Rule implementations
    context.ts      # Shared context for rule checking
    layout.ts       # Layout tree validation
    forbid-paths.ts # Forbidden path patterns
    forbid-names.ts # Forbidden file names
    dependencies.ts # File dependency validation
    mirror.ts       # Mirror structure validation
    when.ts         # Conditional requirements
  output/           # Output formatters
    formatters.ts   # Console, JSON, SARIF formats
  cache/            # File caching system
  types/            # Type definitions with @effect/schema
  errors.ts         # Tagged error types
  version.ts        # Version from package.json
  index.ts          # Public API exports
test/               # Tests (flat structure, *.test.ts)
```

## Development Workflow

When making changes to this project, always follow these steps:

1. **Add Tests** - Write tests for any new features or bug fixes
2. **Update Changelog** - Add notes to CHANGELOG.md describing your changes
3. **Run Validation** - Execute `bun test` and `bun run lint` to ensure all checks pass
4. **Create a PR** - Once all validation passes, create a pull request for review

## Commit Convention

Uses [Conventional Commits](https://www.conventionalcommits.org/):

```bash
feat: ...     # Minor release
fix: ...      # Patch release
perf: ...     # Patch release
feat!: ...    # Major release (breaking)
docs: ...     # No release
chore: ...    # No release
```

## Rules

### layout
Validates file system structure against a tree definition with node types:
- `file` - Single file
- `dir` - Directory with children
- `param` - Dynamic named entries (like Next.js routes)
- `many` - Multiple files matching pattern
- `recursive` - Recursive directory structure
- `either` - One of multiple variants

### forbidPaths
Forbids files matching glob patterns

### forbidNames
Forbids specific file names

### dependencies
Requires certain files to exist when others exist

### mirror
Requires mirrored file structure (e.g., src/*.ts → test/*.test.ts)

### when
Conditional requirements (if X exists, Y must exist)

## Configuration

Config file: `.repo-lint.yaml` or `repo-lint.config.yaml`

```yaml
mode: strict  # or "warn"

ignore:
  - "node_modules/**"
  - "dist/**"

useGitignore: true

layout:
  type: dir
  children:
    src:
      type: dir
      required: true
      children:
        "index.ts": {}
        $files:
          type: many
          pattern: "*.ts"
          case: kebab

scan:
  maxDepth: 100
  maxFiles: 100000
  followSymlinks: false
  timeoutMs: 30000
  concurrency: 10

rules:
  forbidPaths:
    - "**/temp/**"
  forbidNames:
    - ".DS_Store"
  mirror:
    - source: "src/**/*.ts"
      target: "test/**/*.test.ts"
```

## Error Types

All errors extend `Data.TaggedError` for Effect TS compatibility:

- `ConfigNotFoundError` - No config file found
- `ConfigParseError` - YAML parsing failed
- `ConfigValidationError` - Schema validation failed
- `CircularExtendsError` - Circular extends chain detected
- `PathTraversalError` - Path escape attempt in extends
- `FileSystemError` - FS operation failed
- `ScanError` - Directory scanning failed
- `SymlinkLoopError` - Symlink loop detected
- `MaxDepthExceededError` - Directory too deep
- `MaxFilesExceededError` - Too many files
