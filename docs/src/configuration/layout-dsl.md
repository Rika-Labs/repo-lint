# Layout DSL

The layout DSL lets you define your expected filesystem structure declaratively.

## Core Functions

### `dir(children)`

Define a directory with child nodes.

```typescript
dir({
  src: dir({...}),
  "index.ts": file(),
})
```

### `file(pattern?)`

Define a file. Optionally specify a glob pattern.

```typescript
"index.ts": file(),           // Exact name
$any: file("*.ts"),           // Any .ts file
$readme: file("README.*"),    // README with any extension
```

### `opt(node)`

Mark a node as optional (won't error if missing).

```typescript
tests: opt(dir({})),
"README.md": opt(file()),
```

### `param(options, node)`

Dynamic segment with naming constraints.

```typescript
$module: param({ case: "kebab" }, dir({
  "index.ts": file(),
}))
```

Valid paths: `billing/index.ts`, `user-auth/index.ts`
Invalid paths: `UserAuth/index.ts`, `user_auth/index.ts`

#### Options

| Option | Type | Description |
|--------|------|-------------|
| `case` | string | Required case style |
| `name` | string | Parameter name for reporting |

### `many(options, node)`

Allow multiple matches of the same pattern.

```typescript
routes: dir({
  $route: many({ case: "kebab" }, dir({
    "index.ts": file(),
  }))
})
```

Valid structure:
```
routes/
  users/index.ts
  products/index.ts
  orders/index.ts
```

## Case Styles

| Style | Example | Pattern |
|-------|---------|---------|
| `kebab` | `my-module` | lowercase with hyphens |
| `snake` | `my_module` | lowercase with underscores |
| `camel` | `myModule` | camelCase |
| `pascal` | `MyModule` | PascalCase |
| `any` | anything | no restriction |

## Special Keys

- `$name` - Keys starting with `$` are parameter placeholders
- Use with `param()` or `many()` to match dynamic segments

## Full Example

```typescript
layout: dir({
  src: dir({
    services: dir({
      $module: param({ case: "kebab" }, dir({
        api: dir({
          "index.ts": file(),
          routes: dir({
            v1: dir({
              $resource: many({ case: "kebab" }, dir({
                "index.ts": file(),
              })),
            }),
          }),
        }),
        domain: dir({
          entities: dir({ $any: many(file("*.ts")) }),
        }),
        "README.md": opt(file()),
      })),
    }),
  }),
  tests: opt(dir({})),
})
```
