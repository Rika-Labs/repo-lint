# Contributing to repo-lint

Thank you for your interest in contributing to repo-lint!

## Getting Started

1. Fork the repository
2. Clone your fork:
   ```bash
   git clone https://github.com/YOUR_USERNAME/repo-lint.git
   cd repo-lint
   ```
3. Install dependencies:
   ```bash
   bun install
   ```
4. Create a branch:
   ```bash
   git checkout -b feat/your-feature
   ```

## Development

### Running the CLI

```bash
# Run in development mode
bun run dev

# With arguments
bun run dev -- --help
bun run dev -- init --preset typescript
```

### Running Tests

```bash
# Run all tests
bun test

# Run with coverage
bun test --coverage

# Run specific test file
bun test test/naming.test.ts
```

### Linting

```bash
bun run lint
```

## Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/). Your commits will be validated by commitlint.

### Format

```
<type>: <subject>

[optional body]

[optional footer]
```

### Types

- `feat` - New feature (triggers minor release)
- `fix` - Bug fix (triggers patch release)
- `docs` - Documentation only
- `style` - Code style (formatting)
- `refactor` - Code refactor (no feature/fix)
- `perf` - Performance improvement
- `test` - Adding/updating tests
- `build` - Build system or dependencies
- `ci` - CI configuration
- `chore` - Maintenance tasks
- `revert` - Revert a commit

### Examples

```bash
feat: add support for custom naming patterns
fix: correctly parse dynamic imports
docs: update README with new options
test: add tests for boundary rule
```

## Pull Request Process

1. Ensure all tests pass: `bun test`
2. Ensure linting passes: `bun run lint`
3. Update documentation if needed
4. Create a pull request with a clear description

## Code Style

### Effect TS

```typescript
// Use Effect.gen for workflows
export const doSomething = Effect.gen(function* () {
  const result = yield* someEffect;
  return result;
});

// Use tryPromise for async
export const loadFile = (path: string) =>
  Effect.tryPromise({
    try: () => fs.readFile(path, "utf8"),
    catch: (e) => new FileError(e),
  });

// Errors with tags
export class MyError extends Error {
  readonly _tag = "MyError";
}
```

### TypeScript

- Use strict mode
- Prefer `const` over `let`
- Use template literals
- Use arrow functions
- Avoid `any` - use `unknown` if needed

## Adding New Rules

1. Create rule file in `src/rules/`:
   ```typescript
   import { Effect } from "effect";
   import type { Rule } from "../types/rules.js";

   const validateMyRule = (ctx, config) =>
     Effect.gen(function* () {
       const violations = [];
       // ... validation logic
       return violations;
     });

   export const myRule: Rule = {
     name: "my-rule",
     description: "Description of the rule",
     run: validateMyRule,
   };
   ```

2. Register in `src/rules/index.ts`

3. Add tests in `test/my-rule.test.ts`

4. Update documentation

## Questions?

Open an issue or discussion on GitHub.
