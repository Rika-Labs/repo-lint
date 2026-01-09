import { describe, expect, test } from "bun:test";
import { Effect } from "effect";
import { check } from "../src/rules/index.js";
import type { RepoLintConfig, FileEntry } from "../src/types/index.js";

const makeFiles = (paths: readonly string[]): readonly FileEntry[] =>
  paths.map((p) => ({
    path: `/test/${p}`,
    relativePath: p,
    isDirectory: !p.includes("."),
    isSymlink: false,
    depth: p.split("/").length,
  }));

const runCheck = (config: RepoLintConfig, files: readonly FileEntry[]) =>
  Effect.runPromise(check(config, files));

describe("forbidPaths", () => {
  test("detects forbidden paths", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: { forbidPaths: ["**/utils/**"] },
    };

    const result = await runCheck(config, makeFiles(["src/utils/helper.ts", "src/index.ts"]));

    expect(result.violations.length).toBe(1);
    expect(result.violations[0]?.rule).toBe("forbidPaths");
  });
});

describe("forbidNames", () => {
  test("detects forbidden names", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: { forbidNames: ["temp.ts", "tmp.ts"] },
    };

    const result = await runCheck(config, makeFiles(["src/temp.ts", "src/index.ts"]));

    expect(result.violations.length).toBe(1);
    expect(result.violations[0]?.rule).toBe("forbidNames");
  });
});

describe("layout", () => {
  test("validates simple layout", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      layout: {
        type: "dir",
        children: {
          src: { type: "dir", children: { "index.ts": {} } },
          "package.json": {},
        },
      },
    };

    const result = await runCheck(config, makeFiles(["src", "src/index.ts", "package.json"]));
    expect(result.violations.length).toBe(0);
  });

  test("detects unexpected files in strict mode", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      layout: { type: "dir", children: { "package.json": {} } },
    };

    const result = await runCheck(config, makeFiles(["package.json", "unexpected.ts"]));

    expect(result.violations.length).toBe(1);
    expect(result.violations[0]?.path).toBe("unexpected.ts");
  });

  test("validates many node", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      layout: {
        type: "dir",
        children: {
          src: { type: "dir", children: { $files: { type: "many", pattern: "*.ts" } } },
        },
      },
    };

    const result = await runCheck(config, makeFiles(["src", "src/a.ts", "src/b.ts", "src/c.ts"]));
    expect(result.violations.length).toBe(0);
  });

  test("validates case in param node", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      layout: {
        type: "dir",
        children: {
          components: {
            type: "dir",
            children: {
              $component: {
                type: "param",
                case: "pascal",
                child: { type: "dir", children: { "index.tsx": {} } },
              },
            },
          },
        },
      },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "components",
        "components/Button",
        "components/Button/index.tsx",
        "components/bad-name",
        "components/bad-name/index.tsx",
      ]),
    );

    expect(result.violations.length).toBe(1);
    expect(result.violations[0]?.rule).toBe("naming");
  });

  test("validates file pattern mismatch", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      layout: {
        type: "dir",
        children: {
          "config.js": { pattern: "*.ts" },
        },
      },
    };

    const result = await runCheck(config, makeFiles(["config.js"]));
    expect(result.violations.length).toBe(1);
    expect(result.violations[0]?.rule).toBe("layout");
  });

  test("validates many node min/max constraints", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      layout: {
        type: "dir",
        children: {
          src: {
            type: "dir",
            children: { $files: { type: "many", pattern: "*.ts", min: 2, max: 2 } },
          },
        },
      },
    };

    const result = await runCheck(config, makeFiles(["src", "src/only.ts"]));
    expect(result.violations.length).toBe(1);
    expect(result.violations[0]?.rule).toBe("layout");
  });

  test("validates either node when no variant matches", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      layout: {
        type: "dir",
        children: {
          src: {
            type: "either",
            variants: [
              { type: "file", required: true },
              { type: "dir", children: { "index.ts": {} } },
            ],
          },
        },
      },
    };

    const result = await runCheck(config, makeFiles([]));
    expect(result.violations.length).toBe(1);
    expect(result.violations[0]?.rule).toBe("layout");
  });

  test("enforces strict directories", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      layout: {
        type: "dir",
        children: {
          src: {
            type: "dir",
            strict: true,
            children: { "index.ts": {} },
          },
        },
      },
    };

    const result = await runCheck(
      config,
      makeFiles(["src", "src/index.ts", "src/extra.ts"]),
    );
    expect(result.violations.length).toBe(1);
    expect(result.violations[0]?.rule).toBe("layout");
  });

  test("validates recursive nodes", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      layout: {
        type: "dir",
        children: {
          modules: {
            type: "recursive",
            case: "kebab",
            child: { type: "dir", children: { "index.ts": {} } },
          },
        },
      },
    };

    const result = await runCheck(
      config,
      makeFiles(["modules", "modules/BadName", "modules/BadName/index.ts"]),
    );
    expect(result.violations.length).toBe(1);
    expect(result.violations[0]?.rule).toBe("naming");
  });
});

describe("dependencies", () => {
  test("validates dependencies exist", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: { dependencies: { "src/controllers/*.ts": "src/services/*.ts" } },
    };

    const result = await runCheck(config, makeFiles(["src/controllers", "src/controllers/user.ts"]));
    expect(result.violations.filter((v) => v.rule === "dependencies").length).toBe(1);
  });

  test("passes when dependencies exist", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: { dependencies: { "src/controllers/*.ts": "src/services/*.ts" } },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "src/controllers",
        "src/controllers/user.ts",
        "src/services",
        "src/services/user.ts",
      ]),
    );

    expect(result.violations.filter((v) => v.rule === "dependencies").length).toBe(0);
  });
});

describe("when conditions", () => {
  test("validates when conditions", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: { when: { "controller.ts": { requires: ["service.ts"] } } },
    };

    const result = await runCheck(config, makeFiles(["modules/user", "modules/user/controller.ts"]));
    expect(result.violations.filter((v) => v.rule === "when").length).toBe(1);
  });
});

describe("summary", () => {
  test("returns correct summary", async () => {
    const result = await runCheck({ mode: "strict" }, makeFiles(["src", "src/index.ts"]));

    expect(result.summary.filesChecked).toBe(2);
    expect(result.summary.duration).toBeGreaterThanOrEqual(0);
  });
});
