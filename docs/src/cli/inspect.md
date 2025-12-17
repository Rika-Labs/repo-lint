# repo-lint inspect

Debug and understand your config.

## Usage

```bash
repo-lint inspect <SUBCOMMAND>
```

## Subcommands

### layout

Print the resolved layout tree.

```bash
repo-lint inspect layout
repo-lint inspect layout --json
```

Output:
```
├── src [dir]
│   └── services [dir]
│       └── $module [param: module, case: Kebab]
│           ├── api [dir]
│           │   └── index.ts
│           └── domain [dir]
└── tests [dir] (opt)
```

### path

Check if a path is allowed and why.

```bash
repo-lint inspect path src/services/billing/api/index.ts
```

Output:
```
Path: src/services/billing/api/index.ts

  Status: ALLOWED (param module=billing)

  Expected children at this path:
    - (none - this is a file)
```

For invalid paths:
```bash
repo-lint inspect path src/services/MyModule/api
```

```
Path: src/services/MyModule/api

  Status: DENIED
  Reason: 'MyModule' does not match kebab case for parameter module
```

### rule

Get details about a rule.

```bash
repo-lint inspect rule forbidPaths
```

Output:
```
Rule: forbidPaths

  Description: Forbids files/directories matching specified glob patterns
  Auto-fix: no

  Examples:
    - **/utils/** - forbid utils directories
    - **/*.bak - forbid backup files
```

### deps (M4)

Show import dependencies for a file.

```bash
repo-lint inspect deps src/services/billing/api/routes.ts
```

## Options

| Option | Description |
|--------|-------------|
| `--config <PATH>` | Config file path | `repo-lint.config.ts` |
| `--json` | Output as JSON |

## Use Cases

### Debug Layout Matching

```bash
repo-lint inspect path src/services/new-feature/api/index.ts
```

### Understand Rule Behavior

```bash
repo-lint inspect rule layout
```

### Generate Layout Documentation

```bash
repo-lint inspect layout --json > docs/structure.json
```
