# repo-lint - Project Guidelines

**Repository:** [Rika-Labs/repo-lint](https://github.com/Rika-Labs/repo-lint)

## Overview
Extremely fast repository architecture linter with YAML configuration.

## Tech Stack
- **Runtime**: Bun
- **Core**: Effect TS, @effect/cli
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
repo-lint

# With options
repo-lint --config .repo-lint.yaml --format text --pretty

# Initialize config
repo-lint init --preset typescript

# Config management
repo-lint config show
repo-lint config validate

# Explain rules
repo-lint explain naming
```

## Code Standards

### Effect TS Patterns
- Use `Effect.gen` for generator-based workflows
- Use `Effect.tryPromise` for external async calls with typed errors
- Use `Effect.forEach` with `{ concurrency }` for bounded parallel operations
- Errors should extend `Error` with `_tag` discriminator
- Prefer `Effect.orElseSucceed`, `Effect.catchAll` for error recovery
- Use `Option` for optional values, never `undefined | T`

### TypeScript
- Strict mode enabled
- `noUncheckedIndexedAccess: true`
- All tests must be type-safe (no `any`)
- Use discriminated unions for error types

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
  cli/          # @effect/cli command definitions
  commands/     # Command handlers
  config/       # Configuration loading & validation
  core/         # Scanner, matcher, parser
  rules/        # Rule implementations
  output/       # Formatters (text, json, github)
  cache/        # File caching system
  types/        # Type definitions
  errors.ts     # Error types
  index.ts      # Entry point
test/           # Tests mirror src structure
```

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

### naming
Validates file naming conventions (kebab-case, PascalCase, etc.)

### structure
Validates directory structure, required paths, allowed children

### imports
Validates import statements and forbidden dependencies

### boundaries
Enforces architectural layer boundaries

### custom
User-defined regex-based rules

### limits
Enforces file size limits (lines/bytes)

## Configuration

Config file: `.repo-lint.yaml`

```yaml
version: 1
settings:
  root: "."
  ignore: ["node_modules/**"]
naming:
  rules:
    - pattern: "src/**/*.ts"
      style: kebab-case
structure:
  required:
    - path: "src"
boundaries:
  layers:
    - name: core
      pattern: "src/core/**"
  rules:
    - from: core
      allow: [utils]
```
