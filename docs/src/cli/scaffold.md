# repo-lint scaffold

Generate compliant directory structures.

## Usage

```bash
repo-lint scaffold <TYPE> <NAME> [OPTIONS]
```

## Types

### module

Create a new module following your layout config.

```bash
repo-lint scaffold module billing
repo-lint scaffold module user-auth --base-path src/services
```

## Options

| Option | Description |
|--------|-------------|
| `--dry-run` | Preview changes without creating files |
| `--base-path <PATH>` | Base directory for scaffolding | `src/services` |
| `--config <PATH>` | Config file path | `repo-lint.config.ts` |
| `--json` | Output plan as JSON |

## Examples

### Create Module

```bash
repo-lint scaffold module billing
```

Output:
```
Created directory: src/services/billing
Created directory: src/services/billing/api
Created file: src/services/billing/api/index.ts
Created directory: src/services/billing/domain
Created directory: src/services/billing/domain/entities
```

### Preview Changes

```bash
repo-lint scaffold module billing --dry-run
```

Output:
```
Scaffold plan for module 'billing':
  mkdir src/services/billing
  mkdir src/services/billing/api
  touch src/services/billing/api/index.ts
  mkdir src/services/billing/domain
  mkdir src/services/billing/domain/entities
```

### JSON Output (for AI agents)

```bash
repo-lint scaffold module billing --dry-run --json
```

```json
{
  "actions": [
    { "action": "mkdir", "path": "src/services/billing" },
    { "action": "mkdir", "path": "src/services/billing/api" },
    { "action": "touch", "path": "src/services/billing/api/index.ts" }
  ]
}
```

## How It Works

1. Reads your layout config
2. Finds the module template (`$module` param)
3. Generates required directories and files
4. Skips optional nodes
