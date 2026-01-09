# repo-lint - Repository Architecture Linter

## Overview

**repo-lint** is an extremely fast CLI tool for linting repository architecture. It validates file naming conventions, directory structures, import rules, and architectural boundaries based on a simple YAML configuration.

---

## Tech Stack (Mirrors `parallel`)

| Category | Technology |
|----------|------------|
| **Runtime** | Bun |
| **Language** | TypeScript (strict mode) |
| **Core Framework** | Effect TS, @effect/cli, @effect/platform |
| **Linting** | Oxlint |
| **Testing** | Bun test (target: 90%+ coverage) |
| **Git Hooks** | Husky + lint-staged |
| **Commits** | Conventional Commits via commitlint |

---

## YAML Configuration

### Config File: `.repo-lint.yaml`

```yaml
# .repo-lint.yaml
version: 1

# Global settings
settings:
  root: "."                    # Project root (default: current directory)
  ignore:                      # Global ignore patterns
    - "node_modules/**"
    - "dist/**"
    - ".git/**"
    - "**/*.test.ts"
    - "**/*.spec.ts"

# File naming conventions
naming:
  # Define naming patterns
  patterns:
    kebab-case: "^[a-z][a-z0-9]*(-[a-z0-9]+)*$"
    camelCase: "^[a-z][a-zA-Z0-9]*$"
    PascalCase: "^[A-Z][a-zA-Z0-9]*$"
    SCREAMING_SNAKE: "^[A-Z][A-Z0-9]*(_[A-Z0-9]+)*$"
  
  # Apply patterns to paths
  rules:
    - pattern: "src/**/*.ts"
      style: kebab-case
      message: "Source files must use kebab-case"
    
    - pattern: "src/components/**/*.tsx"
      style: PascalCase
      message: "React components must use PascalCase"
    
    - pattern: "src/constants/**/*.ts"
      style: SCREAMING_SNAKE
      message: "Constant files must use SCREAMING_SNAKE_CASE"

# Directory structure rules
structure:
  # Required directories
  required:
    - path: "src"
      message: "Source directory is required"
    - path: "test"
      message: "Test directory is required"
  
  # Enforce specific structure
  enforce:
    - path: "src/commands"
      must-contain:
        - "*.ts"
      message: "Commands directory must contain TypeScript files"
    
    - path: "src"
      allowed-children:
        - "commands"
        - "cli"
        - "config"
        - "core"
        - "utils"
        - "types"
        - "errors.ts"
        - "index.ts"
      message: "Unexpected directory in src/"

# Import/dependency rules
imports:
  # Forbidden imports
  forbidden:
    - pattern: "src/core/**"
      cannot-import:
        - "src/cli/**"
        - "src/commands/**"
      message: "Core modules cannot import from CLI or commands"
    
    - pattern: "src/**"
      cannot-import:
        - "**/../**"
      message: "No parent directory imports allowed"
  
  # Required imports (ensure certain deps are used correctly)
  enforce:
    - pattern: "src/**/*.ts"
      must-import-from:
        - pattern: "effect"
          via: "effect"  # Ensure importing from 'effect' not sub-paths
      message: "Import Effect from 'effect' package"

# Architectural boundaries (layers)
boundaries:
  layers:
    - name: "cli"
      pattern: "src/cli/**"
    - name: "commands"
      pattern: "src/commands/**"
    - name: "core"
      pattern: "src/core/**"
    - name: "utils"
      pattern: "src/utils/**"
  
  rules:
    # Core cannot depend on higher layers
    - from: "core"
      allow: ["utils"]
      message: "Core can only depend on utils"
    
    # Commands can use core and utils
    - from: "commands"
      allow: ["core", "utils"]
      message: "Commands can only depend on core and utils"
    
    # CLI can use everything
    - from: "cli"
      allow: ["commands", "core", "utils"]

# Custom rules (regex-based)
custom:
  - name: "no-console-in-src"
    pattern: "src/**/*.ts"
    match: "console\\.(log|warn|error)"
    exclude: "src/output/**"
    severity: "warn"
    message: "Avoid console.* in source files (use structured logging)"
  
  - name: "no-any"
    pattern: "src/**/*.ts"
    match: ":\\s*any\\b"
    severity: "error"
    message: "Explicit 'any' type is not allowed"

# File size limits
limits:
  - pattern: "src/**/*.ts"
    max-lines: 300
    max-bytes: 15000
    message: "Files should be under 300 lines"
  
  - pattern: "**/*.json"
    max-bytes: 100000
    message: "JSON files should be under 100KB"
```

---

## CLI Commands

### Primary Commands

```bash
# Lint current directory (uses .repo-lint.yaml)
repo-lint

# Lint with explicit config
repo-lint --config path/to/.repo-lint.yaml

# Lint specific paths
repo-lint --path src/ --path test/

# Initialize a new config file
repo-lint init
repo-lint init --preset typescript
repo-lint init --preset react
repo-lint init --preset effect-ts

# Validate config file
repo-lint validate-config

# Output formats
repo-lint --format text          # Human readable (default)
repo-lint --format json          # Machine readable
repo-lint --format github        # GitHub Actions annotations

# Filtering
repo-lint --rule naming          # Only run naming rules
repo-lint --rule structure       # Only run structure rules
repo-lint --rule imports         # Only run import rules
repo-lint --rule boundaries      # Only run boundary rules
repo-lint --severity error       # Only show errors (skip warnings)

# Fix mode (where possible)
repo-lint --fix                  # Auto-fix fixable issues
repo-lint --fix --dry-run        # Show what would be fixed

# Performance
repo-lint --concurrency 8        # Parallel file processing
repo-lint --cache                # Cache results for unchanged files
repo-lint --cache-dir .cache     # Custom cache directory

# CI Mode
repo-lint --ci                   # Exit code 1 on any error, strict mode
```

### Subcommands

```bash
# Initialize config
repo-lint init
repo-lint init --preset <name>
repo-lint init --force            # Overwrite existing

# Config management
repo-lint config show             # Print resolved config
repo-lint config validate         # Validate config syntax
repo-lint config path             # Print config file path

# Debug/Info
repo-lint --version
repo-lint --help
repo-lint explain <rule-name>     # Explain what a rule does
```

---

## File Structure

```
repo-lint/
├── .github/
│   └── workflows/
│       ├── ci.yml
│       └── release.yml
├── .husky/
│   ├── commit-msg
│   └── pre-commit
├── src/
│   ├── cli/
│   │   ├── commands.ts          # @effect/cli command definitions
│   │   └── options.ts           # Shared CLI options
│   ├── commands/
│   │   ├── lint.ts              # Main lint command handler
│   │   ├── init.ts              # Init command handler
│   │   ├── config.ts            # Config subcommands
│   │   └── explain.ts           # Explain command handler
│   ├── config/
│   │   ├── loader.ts            # YAML config loading
│   │   ├── schema.ts            # Config validation schema
│   │   ├── paths.ts             # Config file path resolution
│   │   └── presets/
│   │       ├── typescript.yaml
│   │       ├── react.yaml
│   │       └── effect-ts.yaml
│   ├── core/
│   │   ├── scanner.ts           # Fast file system scanner
│   │   ├── matcher.ts           # Glob/pattern matching
│   │   └── parser.ts            # Lightweight TS/JS import parser
│   ├── rules/
│   │   ├── naming.ts            # File naming rules
│   │   ├── structure.ts         # Directory structure rules
│   │   ├── imports.ts           # Import/dependency rules
│   │   ├── boundaries.ts        # Architectural boundary rules
│   │   ├── custom.ts            # Custom regex rules
│   │   ├── limits.ts            # File size limit rules
│   │   └── index.ts             # Rule registry
│   ├── output/
│   │   ├── format.ts            # Output formatters
│   │   ├── text.ts              # Text formatter
│   │   ├── json.ts              # JSON formatter
│   │   └── github.ts            # GitHub annotations
│   ├── cache/
│   │   ├── file-cache.ts        # File-based caching
│   │   └── hash.ts              # Content hashing
│   ├── types/
│   │   ├── config.ts            # Config type definitions
│   │   ├── rules.ts             # Rule type definitions
│   │   └── results.ts           # Lint result types
│   ├── errors.ts                # Error types (tagged)
│   └── index.ts                 # Entry point
├── test/
│   ├── fixtures/                # Test fixtures
│   │   ├── valid-project/
│   │   └── invalid-project/
│   ├── helpers.ts               # Test utilities
│   ├── config.test.ts
│   ├── naming.test.ts
│   ├── structure.test.ts
│   ├── imports.test.ts
│   ├── boundaries.test.ts
│   ├── scanner.test.ts
│   └── integration.test.ts
├── .gitignore
├── .repo-lint.yaml              # Dogfooding - lint ourselves
├── bunfig.toml
├── CLAUDE.md
├── commitlint.config.js
├── CONTRIBUTING.md
├── LICENSE
├── oxlint.json
├── package.json
├── README.md
└── tsconfig.json
```

---

## Core Features

### 1. **Blazing Fast Scanning**
- Use Bun's native file system APIs
- Parallel directory traversal with `Effect.forEach` + concurrency
- Early bail-out on ignored paths
- Content hashing for cache invalidation

### 2. **Smart Import Parsing**
- Lightweight regex-based import extraction (no full AST)
- Handle ES modules, CommonJS, dynamic imports
- TypeScript path aliases resolution

### 3. **Incremental Linting**
- Hash-based file cache
- Only re-lint changed files
- Cache stored in `.repo-lint-cache/`

### 4. **Rich Error Messages**
- File path with line numbers
- Rule name and severity
- Custom messages from config
- Suggested fixes where applicable

### 5. **Presets**
- Built-in presets for common setups
- Extendable via `extends` in config
- Community presets via npm packages

---

## Implementation Phases

### Phase 1: Foundation (MVP)
1. Project scaffolding (package.json, tsconfig, oxlint, husky)
2. Basic CLI with @effect/cli
3. YAML config loading with schema validation
4. File naming rules
5. Text output formatter
6. Basic tests

### Phase 2: Core Rules
1. Directory structure rules
2. Import/dependency parsing
3. Import rules
4. Architectural boundary rules
5. JSON output formatter

### Phase 3: Performance & Polish
1. File caching system
2. Concurrent file processing
3. GitHub Actions formatter
4. Fix mode (auto-rename files)
5. Preset system

### Phase 4: Advanced Features
1. Custom regex rules
2. File size limits
3. `repo-lint init` with presets
4. `repo-lint explain` command
5. Watch mode (optional)

---

## Dependencies

```json
{
  "dependencies": {
    "@effect/cli": "^0.73.0",
    "@effect/platform": "^0.94.1",
    "@effect/platform-bun": "^0.87.0",
    "effect": "^3.19.14",
    "yaml": "^2.7.0",
    "picomatch": "^4.0.0"
  },
  "devDependencies": {
    "@commitlint/cli": "^20.3.1",
    "@commitlint/config-conventional": "^20.3.1",
    "@types/bun": "latest",
    "husky": "^9.1.7",
    "lint-staged": "^16.2.7",
    "oxlint": "^1.38.0"
  }
}
```

---

## Effect TS Patterns to Follow

```typescript
// Error types with discriminated unions
export class ConfigError extends Error {
  readonly _tag = "ConfigError";
}

export class RuleError extends Error {
  readonly _tag = "RuleError";
  constructor(
    message: string,
    readonly rule: string,
    readonly file: string,
    readonly line?: number
  ) {
    super(message);
  }
}

// Use Effect.gen for workflows
export const lintFiles = (files: readonly string[], config: Config) =>
  Effect.gen(function* () {
    const results = yield* Effect.forEach(
      files,
      (file) => lintFile(file, config),
      { concurrency: config.concurrency ?? 8 }
    );
    return results.flat();
  });

// Use Effect.tryPromise for async operations
export const loadConfig = (path: string) =>
  Effect.tryPromise({
    try: () => fs.readFile(path, "utf8"),
    catch: (e) => new ConfigError(`Failed to load config: ${e}`)
  });

// Use Option for optional values
export const findConfig = Effect.gen(function* () {
  const paths = [".repo-lint.yaml", ".repo-lint.yml", "repo-lint.yaml"];
  for (const p of paths) {
    const exists = yield* Effect.tryPromise({
      try: () => fs.access(p).then(() => true),
      catch: () => false
    });
    if (exists) return Option.some(p);
  }
  return Option.none();
});
```

---

## Example Output

### Text Format (Default)
```
repo-lint v0.1.0

✗ src/MyComponent.ts
  naming: File must use kebab-case (my-component.ts)

✗ src/utils/HELPERS.ts
  naming: File must use kebab-case (helpers.ts)

✗ src/core/handler.ts:15
  imports: Core modules cannot import from CLI or commands
    → import { runCommand } from '../cli/commands'

✗ src/api/
  structure: Unexpected directory in src/ (allowed: commands, cli, config, core, utils)

⚠ src/index.ts:42
  custom/no-console: Avoid console.* in source files

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Found 4 errors and 1 warning in 0.23s
```

### JSON Format
```json
{
  "version": "0.1.0",
  "success": false,
  "stats": {
    "files": 156,
    "errors": 4,
    "warnings": 1,
    "duration_ms": 230
  },
  "results": [
    {
      "file": "src/MyComponent.ts",
      "rule": "naming",
      "severity": "error",
      "message": "File must use kebab-case",
      "suggestion": "my-component.ts"
    }
  ]
}
```

---

## Next Steps

1. **Create repository** at `rika-labs/repo-lint`
2. **Scaffold project** with Phase 1 structure
3. **Implement CLI skeleton** with @effect/cli
4. **Build config loader** with YAML parsing
5. **Implement naming rules** as first rule set
6. **Add tests** for each component
7. **Iterate** through phases

---

## Success Metrics

- **Speed**: Lint 1000 files in < 1 second
- **Coverage**: 90%+ test coverage
- **Usability**: Clear error messages, helpful suggestions
- **Extensibility**: Easy to add new rules and presets
