# repo-lint

A high-performance filesystem layout linter with TypeScript DSL config.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- **TypeScript DSL Config**: Define filesystem structure using `dir`, `file`, `opt`, `param`, `many`, `recursive`, and `either`
- **High Performance**: Built in Rust with parallel file walking (~950k paths/sec)
- **Recursive Matching**: Handle arbitrary-depth structures like Next.js App Router
- **Framework Presets**: Ready-to-use layouts for Next.js and more
- **Naming Convention Rules**: Enforce kebab-case, snake_case, camelCase, or PascalCase
- **Forbidden Paths/Names**: Block unwanted patterns like `**/utils/**` or temporary files
- **Ignore Paths**: Skip directories entirely (honors .gitignore by default)
- **Multiple Output Formats**: Console, JSON, and SARIF (for GitHub Code Scanning)
- **Better Error Messages**: See exactly which patterns were tried when matching fails
- **Scoped Validation**: Lint only a subtree with `--scope`

## Installation

```bash
npm install @rikalabs/repo-lint
```

```bash
pnpm add @rikalabs/repo-lint
```

```bash
yarn add @rikalabs/repo-lint
```

```bash
bun add @rikalabs/repo-lint
```

This installs both the TypeScript types (for config autocomplete) and the CLI binary.

## Quick Start

1. Create a `repo-lint.config.ts` file:

```typescript
import { defineConfig, dir, file, opt, param, many, recursive } from "@rikalabs/repo-lint";

export default defineConfig({
  mode: "strict",

  layout: dir({
    src: dir({
      app: dir({
        // Recursive matching for Next.js App Router routes
        $routes: recursive(
          param({ case: "kebab" }, dir({
            "page.tsx": opt(file()),
            "layout.tsx": opt(file()),
            "loading.tsx": opt(file()),
          }))
        ),
      }),
      components: opt(dir({
        $component: many({ case: "pascal" }, dir({
          "index.tsx": file(),
        })),
      })),
    }),
  }),

  // Ignore these directories entirely
  ignore: [".git", "node_modules", ".next"],

  rules: {
    forbidPaths: ["**/utils/**"],
    forbidNames: ["temp", "new"],
    ignorePaths: ["**/.turbo/**", "**/dist/**"],
  },
});
```

Or use a preset for common frameworks:

```typescript
import { defineConfig, nextjsAppRouter, nextjsDefaultIgnore } from "@rikalabs/repo-lint";

export default defineConfig({
  mode: "strict",
  layout: nextjsAppRouter({ routeCase: "kebab" }),
  ignore: nextjsDefaultIgnore(),
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
repo-lint check                        # Full check
repo-lint check --scope apps/web       # Only validate a subtree
repo-lint check --changed              # Only changed files (git diff)
repo-lint check --json                 # JSON output
repo-lint check --sarif                # SARIF output for GitHub
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
| `recursive(opts?, node)` | Match arbitrary depth (e.g., Next.js routes) |
| `either(...nodes)` | Match any of the variants (first match wins) |

### Case Styles

- `kebab`: my-module-name
- `snake`: my_module_name
- `camel`: myModuleName
- `pascal`: MyModuleName
- `any`: No restriction

### Config Options

```typescript
export default defineConfig({
  mode: "strict",           // "strict" (errors) or "warn" (warnings only)
  layout: dir({...}),       // Your layout definition
  ignore: [".git", "node_modules"],  // Directories to skip entirely
  useGitignore: true,       // Honor .gitignore files (default: true)
  rules: {
    forbidPaths: ["**/utils/**"],    // Error if path matches
    forbidNames: ["temp", "new"],    // Error if filename matches
    ignorePaths: ["**/.turbo/**"],   // Skip silently (no error)
  },
});
```

### Presets

#### Next.js App Router

The `nextjsAppRouter` preset provides a complete layout for Next.js 13+ App Router projects:

```typescript
import { defineConfig, nextjsAppRouter, nextjsDefaultIgnore, nextjsDefaultIgnorePaths } from "@rikalabs/repo-lint";

export default defineConfig({
  layout: nextjsAppRouter({ 
    routeCase: "kebab",  // Route segment naming (default: "kebab")
    maxDepth: 10,        // Max nesting depth for routes (default: 10)
  }),
  ignore: nextjsDefaultIgnore(),
  rules: {
    ignorePaths: nextjsDefaultIgnorePaths(),
  },
});
```

**What's included:**

- **Recursive route matching** - Routes can nest to any depth (`app/dashboard/settings/profile/page.tsx`)
- **All route file conventions** - `page.tsx`, `layout.tsx`, `loading.tsx`, `error.tsx`, `not-found.tsx`, `template.tsx`, `route.ts`, etc.
- **Both `src/app` and `app`** - Supports either project structure
- **Components** - PascalCase component directories with `index.tsx`
- **Common directories** - `lib/`, `hooks/`, `styles/`, `public/`
- **Config files** - `next.config.js`, `tailwind.config.js`, `tsconfig.json`, etc.

**Helper functions:**

| Function | Returns |
|----------|---------|
| `nextjsDefaultIgnore()` | `[".next", "node_modules", ".turbo", "out", ".vercel"]` |
| `nextjsDefaultIgnorePaths()` | `["**/.next/**", "**/node_modules/**", "**/.turbo/**", ...]` |

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
