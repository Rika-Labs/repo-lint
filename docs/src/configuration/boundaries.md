# Boundaries (M4)

> **Note**: Boundary enforcement will be available in Milestone 4.

Boundaries control how modules can import from each other.

## Configuration

```typescript
boundaries: {
  modules: "src/services/*",              // Module root pattern
  publicApi: "src/services/*/api/index.ts", // Public API file
  forbidDeepImports: true,                // Block imports bypassing API
}
```

## How It Works

With `forbidDeepImports: true`:

```typescript
// Allowed: Import from public API
import { createUser } from "../user/api";

// Forbidden: Deep import bypassing API
import { User } from "../user/domain/entities/user";
```

## deps.allow

Fine-grained import rules:

```typescript
deps: {
  allow: [
    { from: "src/services/*/api/**", to: ["src/services/*/domain/**"] },
    { from: "src/services/*/domain/**", to: [] },
    { from: "src/services/*/infra/**", to: ["src/services/*/domain/**"] },
  ],
}
```

This enforces:
- API layer can import from domain
- Domain layer can't import from other layers
- Infra layer can import from domain

## Use Cases

### Microservices Isolation

```typescript
boundaries: {
  modules: "services/*",
  forbidDeepImports: true,
}
```

### Clean Architecture

```typescript
deps: {
  allow: [
    { from: "**/presentation/**", to: ["**/application/**"] },
    { from: "**/application/**", to: ["**/domain/**"] },
    { from: "**/infrastructure/**", to: ["**/domain/**", "**/application/**"] },
    { from: "**/domain/**", to: [] },
  ],
}
```
