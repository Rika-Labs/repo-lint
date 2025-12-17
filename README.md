# repo-lint

A high-performance filesystem layout linter with TypeScript DSL config.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- **TypeScript DSL Config**: Define filesystem structure using intuitive `dir`, `file`, `opt`, `param`, and `many` functions
- **High Performance**: Built in Rust with parallel file walking (200k+ paths/sec)
- **Strict Layout Enforcement**: Ensure your project structure matches your defined layout
- **Naming Convention Rules**: Enforce kebab-case, snake_case, camelCase, or PascalCase
- **Forbidden Paths/Names**: Block unwanted patterns like `**/utils/**` or temporary files
- **Multiple Output Formats**: Console, JSON, and SARIF (for GitHub Code Scanning)
- **Agent-Friendly**: `--agent` and `--trace` flags for AI/automation integration
- **Scaffolding**: Generate compliant module structures automatically

## Installation

```bash
cargo install repo-lint
```

Or build from source:

```bash
git clone https://github.com/rika-labs/repo-lint.git
cd repo-lint
cargo build --release
```

## Quick Start

1. Create a `repo-lint.config.ts` file:

```typescript
import { defineConfig, dir, file, opt, param, many } from "repo-lint";

export default defineConfig({
  mode: "strict",

  layout: dir({
    src: dir({
      services: dir({
        $module: param({ case: "kebab" }, dir({
          api: dir({
            "index.ts": file(),
          }),
          domain: dir({
            entities: dir({ $any: many(file("*.ts")) }),
          }),
          "README.md": opt(file()),
        })),
      }),
    }),
    tests: opt(dir({})),
  }),

  rules: {
    forbidPaths: ["**/utils/**", "**/*.bak"],
    forbidNames: ["temp", "new", "copy"],
  },
});
```

2. Run the linter:

```bash
repo-lint check
```

## Commands

### `repo-lint check`

Check your project structure against the config.

```bash
repo-lint check                    # Full check
repo-lint check --changed          # Only changed files (PR-fast)
repo-lint check --json             # JSON output
repo-lint check --sarif            # SARIF output for GitHub
repo-lint check --fix              # Auto-fix safe moves/renames
repo-lint check --agent            # Enhanced debugging for AI agents
repo-lint check --trace            # Show rule-by-rule matching
```

### `repo-lint scaffold`

Generate compliant directory structures.

```bash
repo-lint scaffold module billing              # Create module structure
repo-lint scaffold module billing --dry-run    # Preview changes
repo-lint scaffold module billing --json       # JSON output for planning
```

### `repo-lint inspect`

Debug and understand your config.

```bash
repo-lint inspect layout                       # Print resolved layout tree
repo-lint inspect path src/services/billing    # Check if path is allowed
repo-lint inspect rule forbidPaths             # Get rule details
repo-lint inspect deps src/foo.ts              # Show import dependencies (M4)
```

## Config Reference

### DSL Functions

| Function | Description |
|----------|-------------|
| `dir({...})` | Define a directory with children |
| `file(pattern?)` | Define a file, optionally with glob pattern |
| `opt(node)` | Mark a node as optional |
| `param(opts, node)` | Dynamic segment with naming constraints |
| `many(opts, node)` | Allow multiple matches |

### Case Styles

- `kebab`: my-module-name
- `snake`: my_module_name
- `camel`: myModuleName
- `pascal`: MyModuleName
- `any`: No restriction

### Rules

```typescript
rules: {
  forbidPaths: ["**/utils/**", "**/*.{bak,tmp}"],
  forbidNames: ["new", "final", "copy", "tmp"],
}
```

### Boundaries (M4)

```typescript
boundaries: {
  modules: "src/services/*",
  publicApi: "src/services/*/api/index.ts",
  forbidDeepImports: true,
}
```

## Output Formats

### Console (default)

```
error[layout]: unexpected file
  --> src/services/billing/utils/helper.ts
  = note: path matches forbidPaths rule: **/utils/**

error[naming]: invalid directory name
  --> src/services/NewModule/api/index.ts
  = note: expected kebab-case, got PascalCase
```

### JSON (`--json`)

```json
{
  "violations": [
    {
      "path": "src/services/billing/utils/helper.ts",
      "rule": "forbidPaths",
      "message": "path matches forbidden pattern: **/utils/**",
      "severity": "error"
    }
  ],
  "summary": { "total": 1, "errors": 1, "warnings": 0 }
}
```

### SARIF (`--sarif`)

Full SARIF 2.1.0 compliance for GitHub Code Scanning integration.

## Performance

repo-lint is built for speed, processing large codebases in milliseconds.

| Benchmark | Files | Time | Throughput |
|-----------|-------|------|------------|
| Small | 51k | 54ms | 942k/sec |
| Medium | 102k | 107ms | **955k/sec** |
| Large | 204k | 228ms | **894k/sec** |

**500k files in ~0.5 seconds** - comparable to ripgrep's directory traversal speed.

### Performance Optimizations

- **fast-glob**: Zero-allocation glob matching (60% faster than regex-based)
- **compact_str**: 24-byte inline strings (no heap allocation for short paths)
- **crossbeam-channel**: Lock-free parallel result collection
- **Parallel traversal**: Uses all CPU cores via the `ignore` crate
- **Early exits**: Skip processing when no rules match

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

## License

MIT License - see [LICENSE](LICENSE) for details.
# repo-lint
