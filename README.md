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

## Claude Code

Add filesystem linting to Claude Code:

```bash
mkdir -p ~/.claude/skills/repo-lint && curl -so ~/.claude/skills/repo-lint/SKILL.md https://raw.githubusercontent.com/Rika-Labs/repo-lint/main/SKILL.md
```

---

[Contributing](CONTRIBUTING.md) · [MIT License](LICENSE)
