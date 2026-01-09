# repo-lint

High-performance filesystem layout linter with YAML config.

## Features

- **YAML Config**: Simple, readable configuration format
- **Monorepo Support**: Configuration inheritance (`extends`), workspace discovery
- **Layout DSL**: `param`, `many`, `recursive`, `either` for complex patterns
- **Case Validation**: Enforce `kebab-case`, `snake_case`, `camelCase`, `PascalCase`
- **Structural Validation**: Dependencies, mirroring, conditional requirements
- **Framework Presets**: Ready-to-use layouts for Next.js
- **Multiple Outputs**: Console, JSON, SARIF (GitHub Code Scanning)
- **High Performance**: Built with Bun for speed

## Installation

```bash
bun add -D @rikalabs/repo-lint
```

## Quick Start

Create `repo-lint.config.yaml`:

```yaml
mode: strict

ignore:
  - node_modules
  - dist
  - .git

layout:
  type: dir
  children:
    src:
      type: dir
      children:
        "$modules":
          type: many
          case: kebab
          child:
            type: file
            pattern: "*.ts"
    package.json: {}

rules:
  forbidNames:
    - temp
    - tmp
```

Run:

```bash
repo-lint check
```

## Configuration

### Layout Nodes

| Type | Description |
|------|-------------|
| `file` | A file (default if no type specified) |
| `dir` | A directory with children |
| `param` | Dynamic segment with naming constraints |
| `many` | Multiple matches with optional count limits |
| `recursive` | Match arbitrary depth (e.g., Next.js routes) |
| `either` | Match any of the variants |

### Node Properties

```yaml
type: dir           # Node type
optional: true      # Not required to exist
required: true      # Must exist (default for non-optional)
case: kebab         # Case validation: kebab, snake, camel, pascal, any
pattern: "*.ts"     # Glob pattern (supports braces: *.{ts,tsx})
strict: true        # Reject undefined children (dirs only)
maxDepth: 3         # Max nesting depth (recursive only)
max: 10             # Max count (many only)
min: 1              # Min count (many only)
```

### Rules

```yaml
rules:
  # Error if path matches
  forbidPaths:
    - "**/utils/**"
    - "**/helpers/**"

  # Error if filename matches
  forbidNames:
    - temp
    - new

  # Skip silently
  ignorePaths:
    - "**/.turbo/**"

  # Require files when source exists
  dependencies:
    "src/controllers/*.ts": "src/services/*.ts"
    "src/**/*.tsx": "src/**/*.test.tsx"

  # Enforce structural mirroring
  mirror:
    - source: "src/components/*"
      target: "src/components/*.test.tsx"
      pattern: "*.tsx -> *.test.tsx"

  # Conditional requirements
  when:
    "controller.ts":
      requires: ["service.ts", "dto.ts"]
```

### Monorepo Support

#### Config Inheritance

```yaml
# apps/web/repo-lint.config.yaml
extends: "@/repo-lint.config.yaml"

rules:
  forbidPaths:
    - "**/tmp/**"
```

#### Workspace Discovery

```yaml
# Root repo-lint.config.yaml
workspaces:
  - "apps/*"
  - "packages/*"
```

### Presets

```yaml
preset: nextjs
```

## CLI

```bash
# Check filesystem
repo-lint check

# Check specific scope
repo-lint check --scope apps/web

# JSON output
repo-lint check --json

# SARIF output (GitHub Code Scanning)
repo-lint check --sarif

# Inspect config
repo-lint inspect layout
repo-lint inspect rule forbidPaths
```

## Output Formats

### Console

```
error[layout]: unexpected file
  --> src/services/billing/utils/helper.ts
  = note: path matches forbidPaths rule

error[naming]: invalid directory name
  --> src/services/NewModule/api/index.ts
  = expected: kebab-case
  = got: NewModule
  = suggestion: new-module
```

### JSON

```json
{
  "violations": [...],
  "summary": {
    "total": 2,
    "errors": 2,
    "warnings": 0,
    "filesChecked": 150,
    "duration": 45
  }
}
```

### SARIF

Full SARIF 2.1.0 compliance for GitHub Code Scanning integration.

## License

MIT
