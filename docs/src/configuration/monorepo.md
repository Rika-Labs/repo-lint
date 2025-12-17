# Monorepo Support

`repo-lint` is designed to handle large monorepos with ease. It supports configuration inheritance, shared layouts via imports, and automatic discovery of child configurations.

## Config Inheritance (`extends`)

You can share common configurations by using the `extends` property. This is particularly useful for setting up base rules and layouts that every package in your monorepo should follow.

```typescript
// apps/sentinel/repo-lint.config.ts
import { defineConfig } from 'repo-lint';

export default defineConfig({
  extends: '@/repo-lint.config.ts', // Root alias support!
  rules: {
    // Override or add rules specific to this app
    forbidPaths: ['**/tmp/**']
  }
});
```

### Root Alias (`@/`)

You can use the `@/` prefix to refer to the root of your repository (specifically, the directory containing the topmost `repo-lint.config.ts`). This avoids brittle relative paths like `../../../../repo-lint.config.ts`.

When a configuration extends another:
- **Mode**: Inherited if not defined in the child.
- **Rules**: `forbidPaths`, `forbidNames`, and `ignorePaths` are merged.
- **Layout**: Child layout overrides the parent layout if provided.
- **Ignore patterns**: Merged.

## Workspace Import Resolution

`repo-lint` supports ES module `import` statements within your configuration files. This allows you to import layouts or constants from other files or packages.

```typescript
// apps/sentinel/repo-lint.config.ts
import { defineConfig, dir } from 'repo-lint';
import { nextjsAppLayout } from '@intimetec/config/repo-lint/nextjs';

export default defineConfig({
  layout: dir({
    src: nextjsAppLayout,
  })
});
```

The internal parser resolves:
1. **Relative paths**: e.g., `./shared-layout` or `../base-config`.
2. **Node Modules**: Lookups in `node_modules` are supported, making it easy to share configs via internal npm packages.

## Built-in Presets

`repo-lint` comes with built-in presets for common project structures.

### Next.js App Router

The `nextjsPreset` provides a standard layout for Next.js App Router projects, including recursive route matching and case validation.

```typescript
import { defineConfig, nextjsPreset } from 'repo-lint';

export default defineConfig(nextjsPreset({
  routeCase: 'kebab', // Enforce kebab-case for all route segments
}));
```

## Root-Level Orchestration

In a monorepo, you typically have a root `repo-lint.config.ts` that defines where the workspaces are.

```typescript
// repo-lint.config.ts at the repository root
import { defineConfig } from 'repo-lint';

export default defineConfig({
  workspaces: ['apps/*', 'packages/*'],
  rules: {
    // Global rules applied across the whole repo
    forbidNames: ['temp', 'test-data']
  }
});
```

By making the `layout` property optional, the root configuration can focus entirely on global rules and workspace discovery. When you run `repo-lint check` at the root, it will automatically find and run all child configurations found in the specified workspaces.
