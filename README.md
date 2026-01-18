# Repo Lint

[![npm](https://img.shields.io/npm/v/@rikalabs/repo-lint.svg)](https://www.npmjs.com/package/@rikalabs/repo-lint)
[![CI](https://github.com/Rika-Labs/repo-lint/actions/workflows/ci.yml/badge.svg)](https://github.com/Rika-Labs/repo-lint/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A high-performance filesystem layout linter with YAML config.

Enforce directory structure, naming conventions, and architectural rules — right from your terminal.

## Install

```bash
bun add -D @rikalabs/repo-lint
```

## Setup

Create `repo-lint.config.yaml`:

```yaml
mode: strict

ignore:
  - node_modules
  - dist

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

## Check

```bash
repo-lint check
```

With specific scope:

```bash
repo-lint check --scope apps/web
```

## Inspect

```bash
repo-lint inspect layout
```

View a specific rule:

```bash
repo-lint inspect rule forbidPaths
```

## Output Formats

```bash
# Console (default)
repo-lint check

# JSON output
repo-lint check --json

# SARIF (GitHub Code Scanning)
repo-lint check --sarif
```

## Options

```
--scope PATH          Check specific directory
--json                JSON output
--sarif               SARIF output for GitHub
--config PATH         Custom config path
```

Run `repo-lint --help` for full documentation.

## Match Rules

Match rules let you target specific directory patterns and enforce structure requirements without defining the entire filesystem layout tree. This is especially useful for monorepos where you only care about structure in certain directories.

### Basic Example

```yaml
rules:
  match:
    - pattern: "apps/*/api/src/modules/*"
      require: [controller.ts, service.ts, repo.ts]
      allow: [errors.ts, lib]
      strict: true
      case: kebab
```

### Options

| Option | Type | Description |
|--------|------|-------------|
| `pattern` | `string` | Glob pattern to match directories (required) |
| `exclude` | `string[]` | Patterns to exclude from matching |
| `require` | `string[]` | Required files/directories that must exist |
| `allow` | `string[]` | Allowed entries (used with `strict: true`) |
| `forbid` | `string[]` | Forbidden files/directories |
| `strict` | `boolean` | Only `require` + `allow` entries permitted |
| `case` | `string` | Naming convention for matched directory (`kebab`, `snake`, `camel`, `pascal`) |
| `childCase` | `string` | Naming convention for children of matched directories |

### Use Cases

**API module structure:**
```yaml
rules:
  match:
    - pattern: "apps/*/api/src/modules/*"
      require: [controller.ts, service.ts, repo.ts]
      allow: [errors.ts, lib, "*.ts"]
      strict: true
      case: kebab  # module directories must be kebab-case
```

**React component directories:**
```yaml
rules:
  match:
    - pattern: "src/components/*"
      require: [index.tsx]
      case: pascal      # Component dirs: Button, UserCard
      childCase: kebab  # Files inside: index.tsx, styles.css
```

**Forbid test files in production code:**
```yaml
rules:
  match:
    - pattern: "src/**/*"
      forbid: ["*.test.ts", "*.spec.ts", "__tests__"]
```

**Exclude specific directories:**
```yaml
rules:
  match:
    - pattern: "apps/*/modules/*"
      exclude: ["apps/legacy/*", "apps/*/modules/deprecated"]
      require: [index.ts]
```

### Behavior Notes

- **Pattern matches nothing:** A warning is emitted if the pattern doesn't match any directories (likely config typo)
- **Strict mode with no patterns:** If `strict: true` but no `require`/`allow`, ALL entries are rejected
- **Overlapping rules:** If multiple rules match the same directory, ALL rules are applied
- **`case` vs `childCase`:** `case` validates the matched directory name; `childCase` validates its children

## Claude Code

Add filesystem linting to Claude Code:

```bash
mkdir -p ~/.claude/skills/repo-lint && curl -so ~/.claude/skills/repo-lint/SKILL.md https://raw.githubusercontent.com/Rika-Labs/repo-lint/main/SKILL.md
```

---

[Contributing](CONTRIBUTING.md) · [MIT License](LICENSE)
