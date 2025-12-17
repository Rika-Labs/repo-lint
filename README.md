# repo-lint

A high-performance filesystem layout linter with TypeScript DSL config.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- **TypeScript DSL Config**: Define filesystem structure using `directory`, `file`, `optional`, `required`, `param`, `many`, `recursive`, and `either`
- **High Performance**: Built in Rust with parallel file walking (~950k paths/sec)
- **Recursive Matching**: Handle arbitrary-depth structures like Next.js App Router
- **Framework Presets**: Ready-to-use layouts for Next.js and more
- **Naming Convention Rules**: Enforce kebab-case, snake_case, camelCase, or PascalCase on files and directories
- **Structural Validation**: Dependencies, mirroring, and conditional requirements between files
- **Depth & Count Limits**: Control directory nesting depth and file counts
- **Strict Mode**: Reject files not explicitly defined in your layout
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

**Monorepo Support:** When your root config defines `workspaces` or multiple `repo-lint.config.ts` files are found, all workspace configs are automatically discovered and validated. Use `--scope` to filter to a specific workspace.

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
| `directory({...})` | Define a directory with children (alias: `dir`) |
| `file(pattern?)` | Define a file, optionally with glob pattern |
| `file({ pattern, case })` | Define a file with case validation |
| `optional(node)` | Mark a node as optional (alias: `opt`) |
| `required(node)` | Mark a node as required (must exist) |
| `param(opts, node)` | Dynamic segment with naming constraints |
| `many(opts, node)` | Allow multiple matches (supports `max` count) |
| `recursive(opts?, node)` | Match arbitrary depth (e.g., Next.js routes) |
| `either(...nodes)` | Match any of the variants (first match wins) |

### Directory Options

```typescript
directory({
  // Children...
}, {
  strict: true,      // Reject files not defined in layout
  maxDepth: 3,       // Maximum nesting depth
})
```

**Strict Mode Behavior:**
- Without `strict: true`: Files/directories not matching any pattern are allowed (or produce warnings in "warn" mode)
- With `strict: true`: Any file or directory in this directory that doesn't match a defined pattern will be **rejected** with an error
- Use strict mode to ensure only explicitly defined files exist in critical directories

### File Case Validation

```typescript
// Enforce kebab-case filenames
$files: many(file({ pattern: "*.ts", case: "kebab" }))
```

**Pattern + Case Validation Behavior:**
- `file({ pattern: "*.ts", case: "kebab" })` applies case validation **only to files that match the pattern**
- Files NOT matching `*.ts` are handled by other rules in the layout (not rejected by this rule)
- To reject all non-matching files, combine with `strict: true` on the parent directory

Example:
```typescript
directory({
  // Only kebab-case .ts files allowed, everything else rejected
  $files: many(file({ pattern: "*.ts", case: "kebab" })),
}, { strict: true })
```

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

### Structural Validation

#### Dependencies

Require that certain files exist when source files match a pattern:

```typescript
dependencies: {
  "src/controllers/*.ts": "src/services/*.ts",  // Controllers need services
  "src/**/*.tsx": "src/**/*.test.tsx",          // Components need tests
}
```

#### Mirror

Enforce structural mirroring between directories:

```typescript
mirror: [
  {
    source: "src/components/*",
    target: "src/components/*.test.tsx",
    pattern: "*.tsx -> *.test.tsx"
  }
]
```

#### When Conditions

Require related files when a trigger file exists:

```typescript
when: {
  "controller.ts": { requires: ["service.ts", "dto.ts"] },
  "index.tsx": { requires: ["styles.css"] }
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
