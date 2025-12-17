# repo-lint check

Check your project against the config.

## Usage

```bash
repo-lint check [OPTIONS] [PATH]
```

## Arguments

| Argument | Description | Default |
|----------|-------------|---------|
| `PATH` | Directory to check | `.` |

## Options

| Option | Description |
|--------|-------------|
| `--changed` | Only check files changed since `--base` |
| `--base <REF>` | Git ref for `--changed` mode | `HEAD` |
| `--fix` | Auto-fix safe moves/renames |
| `--config <PATH>` | Config file path | `repo-lint.config.ts` |
| `--json` | Output as JSON |
| `--sarif` | Output as SARIF 2.1.0 |
| `--agent` | Enhanced debugging for AI agents |
| `--trace` | Show rule-by-rule matching |

## Examples

### Basic Check

```bash
repo-lint check
```

### PR Mode (Fast)

```bash
repo-lint check --changed
repo-lint check --changed --base origin/main
```

### CI Integration

```bash
repo-lint check --sarif > results.sarif
```

### JSON Output

```bash
repo-lint check --json | jq '.violations | length'
```

### Auto-Fix

```bash
repo-lint check --fix
```

Only safe operations:
- Rename files to match case requirements
- Move files to correct locations

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | No errors (warnings allowed in `warn` mode) |
| 1 | Violations found |
| 2 | Config error or crash |

## Output Formats

### Console (default)

```
error[layout]: unexpected file
  --> src/utils/helper.ts
  = note: path not defined in layout

1 error and 0 warnings found.
```

### JSON

```json
{
  "violations": [...],
  "summary": { "total": 1, "errors": 1, "warnings": 0 }
}
```

### SARIF

SARIF 2.1.0 compatible output for GitHub Code Scanning.
