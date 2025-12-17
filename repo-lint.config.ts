import { defineConfig, dir, file, opt, param, many } from "repo-lint";

export default defineConfig({
  mode: "strict",

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
            "use-cases": dir({ $any: many(file("*.ts")) }),
          }),

          infra: opt(dir({
            db: opt(dir({
              migrations: opt(dir({})),
              "index.ts": file(),
            })),
          })),

          "README.md": opt(file()),
        })),
      }),
    }),

    tests: opt(dir({})),
  }),

  rules: {
    forbidPaths: ["**/utils/**", "**/*.{bak,tmp}", "**/*~"],
    forbidNames: ["new", "final", "copy", "tmp", "old"],
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
