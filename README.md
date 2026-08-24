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

## Glob Patterns

Repo-lint uses [picomatch](https://github.com/micromatch/picomatch) for pattern matching with the following behavior:

### Pattern Syntax

| Pattern | Matches | Does NOT Match |
|---------|---------|----------------|
| `*` | Single path segment | Paths with `/` |
| `**` | Zero or more path segments | — |
| `?` | Single character | Multiple chars |
| `[abc]` | One of a, b, c | Other chars |
| `{a,b}` | Either a or b | Other values |
| `!pattern` | Negation | — |

### Important: `*` vs `**`

```yaml
# * matches ONE segment only
pattern: "modules/*"
# ✓ matches: modules/chat, modules/user
# ✗ does NOT match: modules/chat/stream

# ** matches ZERO OR MORE segments
pattern: "modules/**"
# ✓ matches: modules/chat, modules/chat/stream, modules/a/b/c
```

### Basename Patterns

Patterns without `/` are automatically expanded to match anywhere in the path:

```yaml
# Basename patterns (auto-expanded)
ignore:
  - "*.log"      # Matches debug.log, src/debug.log, a/b/c/app.log
  - "*.d.ts"     # Matches index.d.ts, types/api.d.ts

# Path patterns (NOT expanded)
forbidPaths:
  - "src/*.log"  # Only matches src/debug.log, NOT src/sub/debug.log
```

This makes `ignore` and `forbidPaths` configs work intuitively without requiring `**/` prefixes.

### Cross-Platform Paths

Windows-style backslashes are automatically normalized to forward slashes:

```yaml
# This pattern works on both Windows and Unix
pattern: "src/modules/*"
# Matches: src\modules\chat (Windows) and src/modules/chat (Unix)
```

### Dotfiles

Dotfiles (`.gitignore`, `.env`, etc.) are matched by default.

### Absolute vs Relative Paths

Leading slashes are preserved. An absolute path `/src/file.ts` will NOT match a relative pattern `src/*.ts`:

```yaml
# Relative pattern - matches relative paths only
pattern: "src/*.ts"
# ✓ matches: src/file.ts
# ✗ does NOT match: /src/file.ts

# Absolute pattern - matches absolute paths only  
pattern: "/src/*.ts"
# ✓ matches: /src/file.ts
# ✗ does NOT match: src/file.ts
```

### Unicode Normalization

Paths and patterns are automatically normalized to Unicode NFC form. This ensures that `café.ts` matches regardless of whether it's stored as composed (NFC) or decomposed (NFD) Unicode.

### Brace Expansion

Simple brace patterns are supported, but nested braces are NOT:

```yaml
# ✓ Supported
pattern: "*.{ts,tsx}"  # Expands to *.ts and *.tsx

# ✗ NOT supported (throws error)
pattern: "*.{ts,{js,jsx}}"  # Use multiple patterns instead
```

## Mirror Rules

Mirror rules require one computed target file for every file matching `source`. The optional
`exclude` list removes source matches before the target is computed, which lets overlapping file
suffixes use separate mappings.

```yaml
rules:
  mirror:
    # Ordinary tests. Exclude the more specific lanes because *.test.ts matches all three.
    - source: "test/**/*.test.ts"
      target: "src/**/*.ts"
      pattern: "*.test.ts -> *.ts"
      exclude:
        - "test/**/*.tui.test.ts"
        - "test/**/*.proc.test.ts"

    - source: "test/**/*.tui.test.ts"
      target: "src/**/*.ts"
      pattern: "*.tui.test.ts -> *.ts"

    - source: "test/**/*.proc.test.ts"
      target: "src/**/*.ts"
      pattern: "*.proc.test.ts -> *.ts"
```

This maps all of these test files to `src/auth/auth-command.ts`:

- `test/auth/auth-command.test.ts`
- `test/auth/auth-command.tui.test.ts`
- `test/auth/auth-command.proc.test.ts`

`exclude` is optional, so existing mirror configurations keep their current behavior. Mirror rules
are one-way: add a reverse rule separately if both directions must be enforced.

## Path Prefix Rules

`pathPrefix` requires each matched file name to start with its ancestor directories beneath a
declared root, joined by `-`. Use one entry for each structural root:

```yaml
rules:
  pathPrefix:
    - pattern: "**/src/**/*.ts"
      root: src
    - pattern: "**/test/**/*.ts"
      root: test
```

The leading `**/` makes the same rules work at the repository root and in monorepo packages. For a
`src` root:

- `src/auth/auth-command.ts` passes (`auth` prefix)
- `src/auth/session/auth-session-command.ts` passes (`auth-session` prefix)
- `src/auth/session/session-command.ts` fails
- `src/index.ts` passes because it has no ancestor directory beneath `src`

Test suffixes need no special configuration because the prefix is checked only at the start of the
basename. `root` is a literal path segment; if it occurs more than once, the occurrence nearest the
file is used. A matched file that does not contain the declared root is reported as a violation.

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
