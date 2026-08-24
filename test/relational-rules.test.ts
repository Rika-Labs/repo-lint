import { describe, expect, test } from "bun:test";
import { Effect } from "effect";
import { check } from "../src/rules/index.js";
import type { FileEntry, RepoLintConfig } from "../src/types/index.js";

const makeFiles = (paths: readonly string[]): readonly FileEntry[] =>
  paths.map((path) => ({
    path: `/test/${path}`,
    relativePath: path,
    isDirectory: false,
    isSymlink: false,
    depth: path.split(/[\\/]/).length,
  }));

const runCheck = (config: RepoLintConfig, paths: readonly string[]) =>
  Effect.runPromise(check(config, makeFiles(paths)));

describe("mirror exclusions", () => {
  test("maps ordinary, TUI, and process tests without overlapping false positives", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        mirror: [
          {
            source: "test/**/*.test.ts",
            target: "src/**/*.ts",
            pattern: "*.test.ts -> *.ts",
            exclude: ["test/**/*.tui.test.ts", "test/**/*.proc.test.ts"],
          },
          {
            source: "test/**/*.tui.test.ts",
            target: "src/**/*.ts",
            pattern: "*.tui.test.ts -> *.ts",
          },
          {
            source: "test/**/*.proc.test.ts",
            target: "src/**/*.ts",
            pattern: "*.proc.test.ts -> *.ts",
          },
        ],
      },
    };

    const result = await runCheck(config, [
      "src/auth/auth-command.ts",
      "test/auth/auth-command.test.ts",
      "test/auth/auth-command.tui.test.ts",
      "test/auth/auth-command.proc.test.ts",
    ]);

    expect(result.violations.filter((violation) => violation.rule === "mirror")).toEqual([]);
  });

  test("does not enforce an excluded source", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        mirror: [
          {
            source: "test/**/*.test.ts",
            target: "src/**/*.ts",
            pattern: "*.test.ts -> *.ts",
            exclude: ["test/**/*.tui.test.ts"],
          },
        ],
      },
    };

    const result = await runCheck(config, ["test/auth/auth-command.tui.test.ts"]);

    expect(result.violations.filter((violation) => violation.rule === "mirror")).toEqual([]);
  });

  const suffixMappings = [
    [".test.ts", "*.test.ts -> *.ts"],
    [".tui.test.ts", "*.tui.test.ts -> *.ts"],
    [".proc.test.ts", "*.proc.test.ts -> *.ts"],
  ] as const;

  for (const [suffix, transform] of suffixMappings) {
    test(`maps ${suffix} to the source suffix`, async () => {
      const config: RepoLintConfig = {
        mode: "strict",
        rules: {
          mirror: [
            {
              source: `test/**/*${suffix}`,
              target: "src/**/*.ts",
              pattern: transform,
            },
          ],
        },
      };

      const result = await runCheck(config, [`test/auth/session/auth-session-command${suffix}`]);
      const violations = result.violations.filter((violation) => violation.rule === "mirror");

      expect(violations).toHaveLength(1);
      expect(violations[0]?.expected).toBe("src/auth/session/auth-session-command.ts");
    });
  }

  test("preserves mirror rules without exclusions", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        mirror: [
          {
            source: "src/*.ts",
            target: "test/*.test.ts",
            pattern: "*.ts -> *.test.ts",
          },
        ],
      },
    };

    const result = await runCheck(config, ["src/auth.ts"]);
    const violations = result.violations.filter((violation) => violation.rule === "mirror");

    expect(violations).toHaveLength(1);
    expect(violations[0]?.expected).toBe("test/auth.test.ts");
  });

  test("normalizes Windows paths before computing and finding the target", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        mirror: [
          {
            source: "test/**/*.tui.test.ts",
            target: "src/**/*.ts",
            pattern: "*.tui.test.ts -> *.ts",
          },
        ],
      },
    };

    const result = await runCheck(config, [
      "src\\auth\\auth-command.ts",
      "test\\auth\\auth-command.tui.test.ts",
    ]);

    expect(result.violations.filter((violation) => violation.rule === "mirror")).toEqual([]);
  });
});

describe("pathPrefix", () => {
  const config: RepoLintConfig = {
    mode: "strict",
    rules: {
      pathPrefix: [
        { pattern: "**/src/**/*.ts", root: "src" },
        { pattern: "**/test/**/*.ts", root: "test" },
      ],
    },
  };

  test("accepts nested src and test files with the full directory prefix", async () => {
    const result = await runCheck(config, [
      "src/auth/auth-command.ts",
      "src/auth/session/auth-session-command.ts",
      "test/auth/session/auth-session-command.test.ts",
      "test/auth/session/auth-session-command.tui.test.ts",
      "test/auth/session/auth-session-command.proc.test.ts",
    ]);

    expect(result.violations.filter((violation) => violation.rule === "pathPrefix")).toEqual([]);
  });

  test("reports nested files missing any ancestor segment", async () => {
    const result = await runCheck(config, [
      "src/auth/session/session-command.ts",
      "test/auth/session/session-command.tui.test.ts",
    ]);
    const violations = result.violations.filter((violation) => violation.rule === "pathPrefix");

    expect(violations).toHaveLength(2);
    expect(violations.map((violation) => violation.expected)).toEqual([
      "auth-session",
      "auth-session",
    ]);
  });

  test("accepts files directly under the structural root", async () => {
    const result = await runCheck(config, ["src/index.ts", "test/setup.test.ts"]);

    expect(result.violations.filter((violation) => violation.rule === "pathPrefix")).toEqual([]);
  });

  test("supports monorepo globs and Windows paths", async () => {
    const result = await runCheck(config, [
      "packages/api/src/auth/session/auth-session-command.ts",
      "packages\\api\\test\\auth\\session\\auth-session-command.test.ts",
    ]);

    expect(result.violations.filter((violation) => violation.rule === "pathPrefix")).toEqual([]);
  });

  test("reports a matched path that does not contain its declared root", async () => {
    const invalidConfig: RepoLintConfig = {
      mode: "strict",
      rules: {
        pathPrefix: [{ pattern: "lib/**/*.ts", root: "src" }],
      },
    };

    const result = await runCheck(invalidConfig, ["lib/auth/auth-command.ts"]);
    const violations = result.violations.filter((violation) => violation.rule === "pathPrefix");

    expect(violations).toHaveLength(1);
    expect(violations[0]?.message).toContain('structural root "src" not found');
  });
});

describe("noAncestorPrefix", () => {
  const config: RepoLintConfig = {
    mode: "strict",
    rules: {
      noAncestorPrefix: [
        { pattern: "**/src/**/*.ts", root: "src" },
        { pattern: "**/test/**/*.ts", root: "test" },
      ],
    },
  };

  test("accepts contextual names and files directly under the root", async () => {
    const result = await runCheck(config, [
      "src/model-routing/behavior-mode.ts",
      "src/release/update.ts",
      "src/index.ts",
      "test/model-routing/behavior-mode.test.ts",
      "test/release/update.tui.test.ts",
      "test/setup.proc.test.ts",
    ]);

    expect(
      result.violations.filter((violation) => violation.rule === "noAncestorPrefix"),
    ).toEqual([]);
  });

  test("rejects ancestor names and prefixes across normal and test suffixes", async () => {
    const result = await runCheck(config, [
      "src/release/release.ts",
      "src/model-routing/model-routing-behavior-mode.ts",
      "test/release/release.test.ts",
      "test/release/release-update.tui.test.ts",
      "test/model-routing/model-routing.proc.test.ts",
    ]);
    const violations = result.violations.filter(
      (violation) => violation.rule === "noAncestorPrefix",
    );

    expect(violations).toHaveLength(5);
    expect(violations.map((violation) => violation.got)).toEqual([
      "release.ts",
      "model-routing-behavior-mode.ts",
      "release.test.ts",
      "release-update.tui.test.ts",
      "model-routing.proc.test.ts",
    ]);
  });

  test("checks every ancestor below the root", async () => {
    const result = await runCheck(config, [
      "src/auth/session/auth-command.ts",
      "src/auth/session/session.ts",
      "src/auth/session/command.ts",
    ]);
    const violations = result.violations.filter(
      (violation) => violation.rule === "noAncestorPrefix",
    );

    expect(violations).toHaveLength(2);
    expect(violations.map((violation) => violation.message)).toEqual([
      'file name must not repeat ancestor "auth"',
      'file name must not repeat ancestor "session"',
    ]);
  });

  test("respects file exclusions", async () => {
    const excludeConfig: RepoLintConfig = {
      mode: "strict",
      rules: {
        noAncestorPrefix: [
          {
            pattern: "**/src/**/*.ts",
            root: "src",
            exclude: ["**/*.generated.ts"],
          },
        ],
      },
    };

    const result = await runCheck(excludeConfig, ["src/release/release.generated.ts"]);

    expect(
      result.violations.filter((violation) => violation.rule === "noAncestorPrefix"),
    ).toEqual([]);
  });

  test("uses the nearest root segment and normalizes Windows paths", async () => {
    const result = await runCheck(config, [
      "packages\\src\\core\\src\\release\\core-update.test.ts",
      "packages\\src\\core\\src\\release\\release-update.proc.test.ts",
    ]);
    const violations = result.violations.filter(
      (violation) => violation.rule === "noAncestorPrefix",
    );

    expect(violations).toHaveLength(1);
    expect(violations[0]?.path).toBe(
      "packages/src/core/src/release/release-update.proc.test.ts",
    );
  });

  test("reports a matched path without its declared root", async () => {
    const invalidConfig: RepoLintConfig = {
      mode: "strict",
      rules: {
        noAncestorPrefix: [{ pattern: "lib/**/*.ts", root: "src" }],
      },
    };

    const result = await runCheck(invalidConfig, ["lib/release/update.ts"]);
    const violations = result.violations.filter(
      (violation) => violation.rule === "noAncestorPrefix",
    );

    expect(violations).toHaveLength(1);
    expect(violations[0]?.message).toContain('structural root "src" not found');
  });
});
