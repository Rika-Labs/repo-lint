# Configuration Overview

repo-lint uses a TypeScript config file (`repo-lint.config.ts`) that defines:

1. **Layout** - Expected filesystem structure
2. **Rules** - Forbidden paths and names
3. **Boundaries** - Module import restrictions (M4)
4. **Deps** - Fine-grained import rules (M4)

## Config Structure

```typescript
import { defineConfig, dir, file, opt, param, many } from "repo-lint";

export default defineConfig({
  mode: "strict",      // "strict" or "warn"

  layout: dir({...}),  // Required: filesystem structure

  rules: {             // Optional: forbid rules
    forbidPaths: [],
    forbidNames: [],
  },

  boundaries: {...},   // Optional: module boundaries (M4)

  deps: {...},         // Optional: import rules (M4)
});
```

## Mode

| Mode | Behavior |
|------|----------|
| `strict` | Violations are errors (exit code 1) |
| `warn` | Violations are warnings (exit code 0) |

## Config Location

By default, repo-lint looks for `repo-lint.config.ts` in the current directory.

Override with:

```bash
repo-lint check --config path/to/config.ts
```
