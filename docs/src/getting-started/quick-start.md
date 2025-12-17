# Quick Start

## 1. Create Config

Create `repo-lint.config.ts` in your project root:

```typescript
import { defineConfig, dir, file, opt, param } from "repo-lint";

export default defineConfig({
  mode: "strict",

  layout: dir({
    src: dir({
      "index.ts": file(),
      components: opt(dir({})),
    }),
    tests: opt(dir({})),
    "README.md": file(),
  }),

  rules: {
    forbidPaths: ["**/utils/**"],
    forbidNames: ["temp"],
  },
});
```

## 2. Run Check

```bash
repo-lint check
```

Example output:

```
error[layout]: unexpected file
  --> src/helpers/utils.ts
  = note: path not defined in layout

error[forbidPaths]: forbidden path pattern
  --> src/utils/helper.ts
  = note: path matches forbidden pattern: **/utils/**

2 errors and 0 warnings found.
```

## 3. Add to CI

```yaml
# .github/workflows/lint.yml
name: Lint
on: [push, pull_request]

jobs:
  repo-lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo install repo-lint
      - run: repo-lint check
```

## 4. Use Incremental Mode

For faster PR checks:

```bash
repo-lint check --changed
```

## Next Steps

- [Layout DSL](../configuration/layout-dsl.md) - Learn the full DSL
- [Rules](../configuration/rules.md) - Configure forbid rules
- [CLI Reference](../cli/check.md) - All command options
