# Monorepo Example

A typical monorepo structure with packages and apps.

## Config

```typescript
import { defineConfig, dir, file, opt, param, many } from "repo-lint";

export default defineConfig({
  mode: "strict",

  layout: dir({
    packages: dir({
      $package: param({ case: "kebab" }, dir({
        src: dir({
          "index.ts": file(),
          $any: many(file("*.ts")),
        }),
        "package.json": file(),
        "tsconfig.json": file(),
        "README.md": opt(file()),
      })),
    }),

    apps: dir({
      $app: param({ case: "kebab" }, dir({
        src: dir({}),
        "package.json": file(),
      })),
    }),

    "package.json": file(),
    "turbo.json": opt(file()),
    "pnpm-workspace.yaml": opt(file()),
  }),

  rules: {
    forbidPaths: [
      "**/node_modules/**",
      "**/dist/**",
      "**/.turbo/**",
    ],
    forbidNames: ["temp", "test-backup"],
  },
});
```

## Valid Structure

```
monorepo/
├── packages/
│   ├── ui/
│   │   ├── src/
│   │   │   ├── index.ts
│   │   │   └── button.ts
│   │   ├── package.json
│   │   └── tsconfig.json
│   └── utils/              # Would be blocked by forbidPaths if inside src
│       ├── src/
│       │   └── index.ts
│       └── package.json
├── apps/
│   ├── web/
│   │   ├── src/
│   │   └── package.json
│   └── api/
│       ├── src/
│       └── package.json
├── package.json
└── turbo.json
```
