# repo-lint

**repo-lint** is a high-performance filesystem layout linter that enforces structure, naming conventions, and module boundaries using a TypeScript DSL config.

## Why repo-lint?

Modern codebases grow complex quickly. Without guardrails, teams end up with:

- Inconsistent folder structures across modules
- Naming convention drift (PascalCase vs kebab-case)
- "Utils" directories that become dumping grounds
- Import spaghetti across module boundaries

**repo-lint** solves these by:

1. **Defining structure as code** - Your layout config is version-controlled and reviewed like any other code
2. **Enforcing at CI time** - Catch violations before they merge
3. **Helping AI agents** - The `--agent` flag provides rich context for automated refactoring
4. **Running fast** - Rust-powered parallel traversal handles 500k files in seconds

## Key Concepts

### Layout DSL

Define your expected filesystem structure using TypeScript:

```typescript
layout: dir({
  src: dir({
    services: dir({
      $module: param({ case: "kebab" }, dir({
        api: dir({ "index.ts": file() }),
        domain: dir({}),
      })),
    }),
  }),
})
```

### Rules

Block unwanted patterns:

```typescript
rules: {
  forbidPaths: ["**/utils/**"],
  forbidNames: ["temp", "new"],
}
```

### Boundaries (M4)

Control import relationships between modules:

```typescript
boundaries: {
  modules: "src/services/*",
  forbidDeepImports: true,
}
```

## Next Steps

- [Installation](./getting-started/installation.md)
- [Quick Start](./getting-started/quick-start.md)
- [Layout DSL Reference](./configuration/layout-dsl.md)
