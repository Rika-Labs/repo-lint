import { describe, expect, test, beforeAll, afterAll } from "bun:test";
import { Effect } from "effect";
import { check } from "../src/checker.js";
import { scan } from "../src/scanner.js";
import { loadConfig } from "../src/config.js";
import type { RepoLintConfig } from "../src/types.js";
import { mkdir, writeFile, rm } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";

let monorepoRoot: string;

const createFile = async (path: string, content = "") => {
  const dir = path.substring(0, path.lastIndexOf("/"));
  await mkdir(dir, { recursive: true });
  await writeFile(path, content);
};

const strictMonorepoConfig: RepoLintConfig = {
  mode: "strict",
  ignore: ["node_modules", "dist", ".turbo", ".next", "coverage"],
  layout: {
    type: "dir",
    children: {
      packages: {
        type: "dir",
        children: {
          $package: {
            type: "param",
            case: "kebab",
            child: {
              type: "dir",
              children: {
                src: {
                  type: "dir",
                  children: {
                    "index.ts": { required: true },
                    $files: { type: "many", pattern: "*.ts", case: "kebab" },
                  },
                },
                test: {
                  type: "dir",
                  optional: true,
                  children: {
                    $tests: { type: "many", pattern: "*.test.ts", case: "kebab" },
                  },
                },
                "package.json": { required: true },
                "tsconfig.json": { required: true },
                "README.md": { optional: true },
              },
            },
          },
        },
      },
      apps: {
        type: "dir",
        children: {
          $app: {
            type: "param",
            case: "kebab",
            child: {
              type: "dir",
              children: {
                src: {
                  type: "dir",
                  children: {
                    app: {
                      type: "dir",
                      optional: true,
                      children: {
                        $routes: {
                          type: "recursive",
                          case: "kebab",
                          child: {
                            type: "dir",
                            children: {
                              "page.tsx": { optional: true },
                              "layout.tsx": { optional: true },
                              "loading.tsx": { optional: true },
                              "error.tsx": { optional: true },
                            },
                          },
                        },
                      },
                    },
                    components: {
                      type: "dir",
                      optional: true,
                      children: {
                        $component: {
                          type: "param",
                          case: "pascal",
                          child: {
                            type: "dir",
                            children: {
                              "index.tsx": { required: true },
                              $files: { type: "many", pattern: "*.{ts,tsx,css}", optional: true },
                            },
                          },
                        },
                      },
                    },
                    lib: {
                      type: "dir",
                      optional: true,
                      children: {
                        $files: { type: "many", pattern: "*.ts", case: "kebab" },
                      },
                    },
                    hooks: {
                      type: "dir",
                      optional: true,
                      children: {
                        $hooks: { type: "many", pattern: "use*.ts", case: "camel" },
                      },
                    },
                  },
                },
                public: {
                  type: "dir",
                  optional: true,
                  children: {
                    $assets: { type: "many", optional: true },
                  },
                },
                "package.json": { required: true },
                "tsconfig.json": { required: true },
                "next.config.ts": { optional: true },
                "tailwind.config.ts": { optional: true },
              },
            },
          },
        },
      },
      libs: {
        type: "dir",
        children: {
          $lib: {
            type: "param",
            case: "kebab",
            child: {
              type: "dir",
              children: {
                src: {
                  type: "dir",
                  children: {
                    "index.ts": { required: true },
                    $files: { type: "many", pattern: "*.ts", case: "kebab" },
                  },
                },
                test: {
                  type: "dir",
                  optional: true,
                  children: {
                    $tests: { type: "many", pattern: "*.test.ts", case: "kebab" },
                  },
                },
                "package.json": { required: true },
                "tsconfig.json": { required: true },
              },
            },
          },
        },
      },
      tooling: {
        type: "dir",
        optional: true,
        children: {
          $tool: {
            type: "param",
            case: "kebab",
            child: {
              type: "dir",
              children: {
                "index.js": { optional: true },
                "index.ts": { optional: true },
                "package.json": { required: true },
              },
            },
          },
        },
      },
      "package.json": { required: true },
      "turbo.json": { optional: true },
      "pnpm-workspace.yaml": { optional: true },
      "tsconfig.json": { optional: true },
      ".gitignore": { optional: true },
      "README.md": { optional: true },
    },
  },
  rules: {
    forbidPaths: [
      "**/node_modules/**",
      "**/.git/**",
      "**/dist/**",
      "**/.turbo/**",
      "**/coverage/**",
      "**/__snapshots__/**",
    ],
    forbidNames: [
      "temp.ts",
      "tmp.ts",
      "test.ts",
      "foo.ts",
      "bar.ts",
      "baz.ts",
      "index.js",
      "Untitled.ts",
    ],
    ignorePaths: ["**/node_modules/**", "**/dist/**"],
    dependencies: {
      "apps/*/src/**/*.tsx": "libs/ui/src/index.ts",
      "packages/*/src/**/*.ts": "libs/utils/src/index.ts",
    },
    mirror: [
      {
        source: "packages/*/src/*.ts",
        target: "packages/*/test/*.test.ts",
        pattern: "*.ts -> *.test.ts",
      },
      {
        source: "libs/*/src/*.ts",
        target: "libs/*/test/*.test.ts",
        pattern: "*.ts -> *.test.ts",
      },
    ],
    when: {
      "src/index.ts": { requires: ["package.json", "tsconfig.json"] },
    },
  },
};

beforeAll(async () => {
  monorepoRoot = join(tmpdir(), `repo-lint-monorepo-test-${Date.now()}`);

  await createFile(join(monorepoRoot, "package.json"), '{"name": "monorepo"}');
  await createFile(join(monorepoRoot, "turbo.json"), "{}");
  await createFile(join(monorepoRoot, "pnpm-workspace.yaml"), "packages:\n  - packages/*");
  await createFile(join(monorepoRoot, "tsconfig.json"), "{}");
  await createFile(join(monorepoRoot, ".gitignore"), "node_modules");
  await createFile(join(monorepoRoot, "README.md"), "# Monorepo");

  await createFile(join(monorepoRoot, "packages/api/package.json"), '{"name": "@repo/api"}');
  await createFile(join(monorepoRoot, "packages/api/tsconfig.json"), "{}");
  await createFile(join(monorepoRoot, "packages/api/src/index.ts"), "export {}");
  await createFile(join(monorepoRoot, "packages/api/src/router.ts"), "export {}");
  await createFile(join(monorepoRoot, "packages/api/src/handlers.ts"), "export {}");
  await createFile(join(monorepoRoot, "packages/api/test/router.test.ts"), "test('', () => {})");
  await createFile(join(monorepoRoot, "packages/api/README.md"), "# API");

  await createFile(join(monorepoRoot, "packages/web/package.json"), '{"name": "@repo/web"}');
  await createFile(join(monorepoRoot, "packages/web/tsconfig.json"), "{}");
  await createFile(join(monorepoRoot, "packages/web/src/index.ts"), "export {}");

  await createFile(join(monorepoRoot, "packages/shared/package.json"), '{"name": "@repo/shared"}');
  await createFile(join(monorepoRoot, "packages/shared/tsconfig.json"), "{}");
  await createFile(join(monorepoRoot, "packages/shared/src/index.ts"), "export {}");
  await createFile(join(monorepoRoot, "packages/shared/src/types.ts"), "export {}");
  await createFile(join(monorepoRoot, "packages/shared/test/types.test.ts"), "test('', () => {})");

  await createFile(join(monorepoRoot, "apps/dashboard/package.json"), '{"name": "@repo/dashboard"}');
  await createFile(join(monorepoRoot, "apps/dashboard/tsconfig.json"), "{}");
  await createFile(join(monorepoRoot, "apps/dashboard/next.config.ts"), "export default {}");
  await createFile(join(monorepoRoot, "apps/dashboard/src/app/page.tsx"), "export default () => null");
  await createFile(join(monorepoRoot, "apps/dashboard/src/app/layout.tsx"), "export default () => null");
  await createFile(join(monorepoRoot, "apps/dashboard/src/app/settings/page.tsx"), "export default () => null");
  await createFile(join(monorepoRoot, "apps/dashboard/src/components/Button/index.tsx"), "export {}");
  await createFile(join(monorepoRoot, "apps/dashboard/src/components/Card/index.tsx"), "export {}");
  await createFile(join(monorepoRoot, "apps/dashboard/src/lib/api.ts"), "export {}");
  await createFile(join(monorepoRoot, "apps/dashboard/src/hooks/useAuth.ts"), "export {}");

  await createFile(join(monorepoRoot, "apps/marketing/package.json"), '{"name": "@repo/marketing"}');
  await createFile(join(monorepoRoot, "apps/marketing/tsconfig.json"), "{}");
  await createFile(join(monorepoRoot, "apps/marketing/src/app/page.tsx"), "export default () => null");
  await createFile(join(monorepoRoot, "apps/marketing/src/components/Hero/index.tsx"), "export {}");

  await createFile(join(monorepoRoot, "libs/ui/package.json"), '{"name": "@repo/ui"}');
  await createFile(join(monorepoRoot, "libs/ui/tsconfig.json"), "{}");
  await createFile(join(monorepoRoot, "libs/ui/src/index.ts"), "export {}");
  await createFile(join(monorepoRoot, "libs/ui/src/button.ts"), "export {}");
  await createFile(join(monorepoRoot, "libs/ui/src/card.ts"), "export {}");
  await createFile(join(monorepoRoot, "libs/ui/test/button.test.ts"), "test('', () => {})");

  await createFile(join(monorepoRoot, "libs/utils/package.json"), '{"name": "@repo/utils"}');
  await createFile(join(monorepoRoot, "libs/utils/tsconfig.json"), "{}");
  await createFile(join(monorepoRoot, "libs/utils/src/index.ts"), "export {}");
  await createFile(join(monorepoRoot, "libs/utils/src/format.ts"), "export {}");

  await createFile(join(monorepoRoot, "libs/config/package.json"), '{"name": "@repo/config"}');
  await createFile(join(monorepoRoot, "libs/config/tsconfig.json"), "{}");
  await createFile(join(monorepoRoot, "libs/config/src/index.ts"), "export {}");

  await createFile(join(monorepoRoot, "tooling/eslint/package.json"), '{"name": "@repo/eslint-config"}');
  await createFile(join(monorepoRoot, "tooling/eslint/index.js"), "module.exports = {}");

  await createFile(join(monorepoRoot, "tooling/typescript/package.json"), '{"name": "@repo/typescript-config"}');
  await createFile(join(monorepoRoot, "tooling/typescript/index.ts"), "export {}");
});

afterAll(async () => {
  await rm(monorepoRoot, { recursive: true, force: true });
});

describe("monorepo structure validation", () => {
  test("valid monorepo passes all checks", async () => {
    const files = await Effect.runPromise(scan({ root: monorepoRoot, ignore: ["node_modules"] }));
    const result = await Effect.runPromise(check(strictMonorepoConfig, files));

    const structureViolations = result.violations.filter(
      (v) => v.rule === "layout" || v.rule === "naming"
    );

    expect(structureViolations).toEqual([]);
  });

  test("detects invalid package naming (PascalCase instead of kebab)", async () => {
    const badPackageDir = join(monorepoRoot, "packages/BadPackage");
    await createFile(join(badPackageDir, "package.json"), "{}");
    await createFile(join(badPackageDir, "tsconfig.json"), "{}");
    await createFile(join(badPackageDir, "src/index.ts"), "export {}");

    const files = await Effect.runPromise(scan({ root: monorepoRoot, ignore: ["node_modules"] }));
    const result = await Effect.runPromise(check(strictMonorepoConfig, files));

    const namingViolations = result.violations.filter(
      (v) => v.rule === "naming" && v.path.includes("BadPackage")
    );

    expect(namingViolations.length).toBeGreaterThan(0);
    expect(namingViolations[0]?.message).toContain("kebab-case");

    await rm(badPackageDir, { recursive: true, force: true });
  });

  test("detects invalid component naming (kebab instead of PascalCase)", async () => {
    const badComponentDir = join(monorepoRoot, "apps/dashboard/src/components/bad-button");
    await createFile(join(badComponentDir, "index.tsx"), "export {}");

    const files = await Effect.runPromise(scan({ root: monorepoRoot, ignore: ["node_modules"] }));
    const result = await Effect.runPromise(check(strictMonorepoConfig, files));

    const namingViolations = result.violations.filter(
      (v) => v.rule === "naming" && v.path.includes("bad-button")
    );

    expect(namingViolations.length).toBeGreaterThan(0);
    expect(namingViolations[0]?.message).toContain("PascalCase");

    await rm(badComponentDir, { recursive: true, force: true });
  });

  test("detects forbidden file names", async () => {
    const tempFile = join(monorepoRoot, "packages/api/src/temp.ts");
    await createFile(tempFile, "export {}");

    const files = await Effect.runPromise(scan({ root: monorepoRoot, ignore: ["node_modules"] }));
    const result = await Effect.runPromise(check(strictMonorepoConfig, files));

    const forbidViolations = result.violations.filter(
      (v) => v.rule === "forbidNames" && v.path.includes("temp.ts")
    );

    expect(forbidViolations.length).toBe(1);

    await rm(tempFile, { force: true });
  });

  test("detects missing required files", async () => {
    const incompletePackage = join(monorepoRoot, "packages/incomplete");
    await mkdir(join(incompletePackage, "src"), { recursive: true });
    await createFile(join(incompletePackage, "package.json"), "{}");

    const files = await Effect.runPromise(scan({ root: monorepoRoot, ignore: ["node_modules"] }));
    const result = await Effect.runPromise(check(strictMonorepoConfig, files));

    const layoutViolations = result.violations.filter(
      (v) => v.path.includes("incomplete") && v.rule === "layout"
    );

    expect(layoutViolations.length).toBeGreaterThan(0);

    await rm(incompletePackage, { recursive: true, force: true });
  });

  test("detects unexpected files in strict mode", async () => {
    const unexpectedFile = join(monorepoRoot, "random-file.ts");
    await createFile(unexpectedFile, "export {}");

    const files = await Effect.runPromise(scan({ root: monorepoRoot, ignore: ["node_modules"] }));
    const result = await Effect.runPromise(check(strictMonorepoConfig, files));

    const layoutViolations = result.violations.filter(
      (v) => v.rule === "layout" && v.path === "random-file.ts"
    );

    expect(layoutViolations.length).toBe(1);
    expect(layoutViolations[0]?.message).toContain("unexpected");

    await rm(unexpectedFile, { force: true });
  });
});

describe("monorepo hooks validation", () => {
  test("detects invalid hook naming (not starting with use)", async () => {
    const badHook = join(monorepoRoot, "apps/dashboard/src/hooks/getAuth.ts");
    await createFile(badHook, "export {}");

    const files = await Effect.runPromise(scan({ root: monorepoRoot, ignore: ["node_modules"] }));
    const result = await Effect.runPromise(check(strictMonorepoConfig, files));

    const hookViolations = result.violations.filter(
      (v) => v.path.includes("getAuth") && v.rule === "layout"
    );

    expect(hookViolations.length).toBeGreaterThan(0);

    await rm(badHook, { force: true });
  });

  test("valid hook naming passes", async () => {
    const goodHook = join(monorepoRoot, "apps/dashboard/src/hooks/useData.ts");
    await createFile(goodHook, "export {}");

    const files = await Effect.runPromise(scan({ root: monorepoRoot, ignore: ["node_modules"] }));
    const result = await Effect.runPromise(check(strictMonorepoConfig, files));

    const hookViolations = result.violations.filter(
      (v) => v.path.includes("useData") && (v.rule === "naming" || v.rule === "layout")
    );

    expect(hookViolations).toEqual([]);

    await rm(goodHook, { force: true });
  });
});

describe("monorepo app routes validation", () => {
  test("valid nested routes pass", async () => {
    const nestedRoute = join(monorepoRoot, "apps/dashboard/src/app/users/profile/page.tsx");
    await createFile(nestedRoute, "export default () => null");

    const files = await Effect.runPromise(scan({ root: monorepoRoot, ignore: ["node_modules"] }));
    const result = await Effect.runPromise(check(strictMonorepoConfig, files));

    const routeViolations = result.violations.filter(
      (v) => v.path.includes("users/profile")
    );

    expect(routeViolations).toEqual([]);

    await rm(join(monorepoRoot, "apps/dashboard/src/app/users"), { recursive: true, force: true });
  });

  test("detects invalid route naming (PascalCase)", async () => {
    const badRoute = join(monorepoRoot, "apps/dashboard/src/app/UserSettings/page.tsx");
    await createFile(badRoute, "export default () => null");

    const files = await Effect.runPromise(scan({ root: monorepoRoot, ignore: ["node_modules"] }));
    const result = await Effect.runPromise(check(strictMonorepoConfig, files));

    const routeViolations = result.violations.filter(
      (v) => v.path.includes("UserSettings") && v.rule === "naming"
    );

    expect(routeViolations.length).toBeGreaterThan(0);
    expect(routeViolations[0]?.message).toContain("kebab-case");

    await rm(join(monorepoRoot, "apps/dashboard/src/app/UserSettings"), { recursive: true, force: true });
  });
});

describe("monorepo mirror rules", () => {
  test("detects missing test files for package sources", async () => {
    const sourceFile = join(monorepoRoot, "packages/api/src/validation.ts");
    await createFile(sourceFile, "export {}");

    const files = await Effect.runPromise(scan({ root: monorepoRoot, ignore: ["node_modules"] }));
    const result = await Effect.runPromise(check(strictMonorepoConfig, files));

    const mirrorViolations = result.violations.filter(
      (v) => v.rule === "mirror" && v.path.includes("validation.ts")
    );

    expect(mirrorViolations.length).toBe(1);
    expect(mirrorViolations[0]?.expected).toContain("validation.test.ts");

    await rm(sourceFile, { force: true });
  });

  test("passes when test file exists", async () => {
    const sourceFile = join(monorepoRoot, "packages/api/src/auth.ts");
    const testFile = join(monorepoRoot, "packages/api/test/auth.test.ts");
    await createFile(sourceFile, "export {}");
    await createFile(testFile, "test('', () => {})");

    const files = await Effect.runPromise(scan({ root: monorepoRoot, ignore: ["node_modules"] }));
    const result = await Effect.runPromise(check(strictMonorepoConfig, files));

    const mirrorViolations = result.violations.filter(
      (v) => v.rule === "mirror" && v.path.includes("auth.ts")
    );

    expect(mirrorViolations).toEqual([]);

    await rm(sourceFile, { force: true });
    await rm(testFile, { force: true });
  });
});

describe("monorepo lib file naming", () => {
  test("detects invalid lib file naming (camelCase)", async () => {
    const badFile = join(monorepoRoot, "apps/dashboard/src/lib/apiClient.ts");
    await createFile(badFile, "export {}");

    const files = await Effect.runPromise(scan({ root: monorepoRoot, ignore: ["node_modules"] }));
    const result = await Effect.runPromise(check(strictMonorepoConfig, files));

    const namingViolations = result.violations.filter(
      (v) => v.path.includes("apiClient") && v.rule === "naming"
    );

    expect(namingViolations.length).toBeGreaterThan(0);
    expect(namingViolations[0]?.message).toContain("kebab-case");

    await rm(badFile, { force: true });
  });

  test("valid lib file naming passes", async () => {
    const goodFile = join(monorepoRoot, "apps/dashboard/src/lib/api-client.ts");
    await createFile(goodFile, "export {}");

    const files = await Effect.runPromise(scan({ root: monorepoRoot, ignore: ["node_modules"] }));
    const result = await Effect.runPromise(check(strictMonorepoConfig, files));

    const namingViolations = result.violations.filter(
      (v) => v.path.includes("api-client") && v.rule === "naming"
    );

    expect(namingViolations).toEqual([]);

    await rm(goodFile, { force: true });
  });
});

describe("monorepo config file loading", () => {
  test("loads and validates monorepo config from YAML", async () => {
    const configPath = join(monorepoRoot, "repo-lint.config.yaml");
    const configContent = `
mode: strict
ignore:
  - node_modules
  - dist
layout:
  type: dir
  children:
    packages:
      type: dir
      children:
        $package:
          type: param
          case: kebab
          child:
            type: dir
            children:
              src:
                type: dir
                children:
                  "index.ts":
                    required: true
              "package.json":
                required: true
rules:
  forbidNames:
    - temp.ts
    - tmp.ts
`;
    await createFile(configPath, configContent);

    const config = await Effect.runPromise(loadConfig(configPath));

    expect(config.mode).toBe("strict");
    expect(config.layout?.children?.["packages"]).toBeDefined();
    expect(config.rules?.forbidNames).toContain("temp.ts");

    await rm(configPath, { force: true });
  });
});

describe("monorepo summary statistics", () => {
  test("provides accurate summary for large monorepo", async () => {
    const files = await Effect.runPromise(scan({ root: monorepoRoot, ignore: ["node_modules"] }));
    const result = await Effect.runPromise(check(strictMonorepoConfig, files));

    expect(result.summary.filesChecked).toBeGreaterThan(40);
    expect(result.summary.duration).toBeLessThan(1000);
    expect(typeof result.summary.errors).toBe("number");
    expect(typeof result.summary.warnings).toBe("number");
  });
});
