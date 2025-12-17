# Rules

Rules let you forbid certain paths and names across your project.

## forbidPaths

Block files/directories matching glob patterns.

```typescript
rules: {
  forbidPaths: [
    "**/utils/**",       // No utils directories
    "**/*.bak",          // No backup files
    "**/*.{tmp,temp}",   // No temp files
    "**/node_modules/**",// No node_modules (usually gitignored anyway)
    "**/__pycache__/**", // No Python cache
  ],
}
```

### Glob Syntax

| Pattern | Matches |
|---------|---------|
| `*` | Any single segment |
| `**` | Any number of segments |
| `?` | Any single character |
| `[abc]` | Character class |
| `{a,b}` | Alternatives |

## forbidNames

Block files/directories with specific names (case-insensitive).

```typescript
rules: {
  forbidNames: [
    "temp",    // temp.ts, TEMP/, Temp.js
    "new",     // new.ts, NEW/
    "copy",    // copy.ts, copy (2).ts
    "final",   // final.ts, final-v2.ts
    "old",     // old.ts, old/
    "backup",  // backup.ts, backup/
  ],
}
```

## Common Patterns

### Clean Architecture

```typescript
rules: {
  forbidPaths: [
    "**/utils/**",
    "**/helpers/**",
    "**/common/**",
  ],
  forbidNames: ["misc", "stuff", "other"],
}
```

### No Temporary Files

```typescript
rules: {
  forbidPaths: [
    "**/*.{bak,tmp,temp}",
    "**/*~",
    "**/.DS_Store",
  ],
  forbidNames: ["temp", "tmp", "backup"],
}
```

### Enforce Lowercase

Combined with layout `param({ case: "kebab" })`:

```typescript
rules: {
  forbidNames: [],  // Naming enforced by layout
}
```
