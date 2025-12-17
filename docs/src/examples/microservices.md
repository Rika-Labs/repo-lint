# Microservices Example

A microservices architecture with clean boundaries.

## Config

```typescript
import { defineConfig, dir, file, opt, param, many } from "repo-lint";

export default defineConfig({
  mode: "strict",

  layout: dir({
    src: dir({
      services: dir({
        $service: param({ case: "kebab" }, dir({
          api: dir({
            "index.ts": file(),
            routes: dir({
              v1: dir({
                $resource: many({ case: "kebab" }, dir({
                  "index.ts": file(),
                  $handler: many(file("*.handler.ts")),
                })),
              }),
            }),
            middleware: opt(dir({})),
          }),

          domain: dir({
            entities: dir({ $any: many(file("*.ts")) }),
            "use-cases": dir({ $any: many(file("*.ts")) }),
            repositories: dir({ $any: many(file("*.ts")) }),
          }),

          infra: opt(dir({
            db: opt(dir({
              migrations: opt(dir({})),
              "index.ts": file(),
            })),
            cache: opt(dir({})),
            queue: opt(dir({})),
          })),

          "README.md": opt(file()),
        })),
      }),

      shared: opt(dir({
        types: opt(dir({})),
        constants: opt(dir({})),
      })),
    }),

    tests: opt(dir({
      integration: opt(dir({})),
      e2e: opt(dir({})),
    })),
  }),

  rules: {
    forbidPaths: [
      "**/utils/**",
      "**/helpers/**",
      "**/common/**",
      "**/*.{bak,tmp}",
    ],
    forbidNames: ["new", "final", "copy", "temp"],
  },

  boundaries: {
    modules: "src/services/*",
    publicApi: "src/services/*/api/index.ts",
    forbidDeepImports: true,
  },

  deps: {
    allow: [
      { from: "src/services/*/api/**", to: ["src/services/*/domain/**"] },
      { from: "src/services/*/domain/**", to: [] },
      { from: "src/services/*/infra/**", to: ["src/services/*/domain/**"] },
    ],
  },
});
```

## Valid Structure

```
src/
└── services/
    ├── billing/
    │   ├── api/
    │   │   ├── index.ts
    │   │   └── routes/
    │   │       └── v1/
    │   │           ├── invoices/
    │   │           │   ├── index.ts
    │   │           │   ├── create.handler.ts
    │   │           │   └── list.handler.ts
    │   │           └── payments/
    │   │               └── index.ts
    │   ├── domain/
    │   │   ├── entities/
    │   │   │   ├── invoice.ts
    │   │   │   └── payment.ts
    │   │   ├── use-cases/
    │   │   │   ├── create-invoice.ts
    │   │   │   └── process-payment.ts
    │   │   └── repositories/
    │   │       └── invoice-repository.ts
    │   ├── infra/
    │   │   └── db/
    │   │       ├── index.ts
    │   │       └── migrations/
    │   └── README.md
    └── user-auth/
        ├── api/
        │   └── index.ts
        └── domain/
            └── entities/
                └── user.ts
```
