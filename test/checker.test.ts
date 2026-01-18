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

describe("match rules", () => {
  // =========================================================================
  // Basic functionality
  // =========================================================================

  test("validates required files in matched directories", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        match: [
          {
            pattern: "apps/*/api/src/modules/*",
            require: ["controller.ts", "service.ts", "repo.ts"],
          },
        ],
      },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "apps/sentinel/api/src/modules/user",
        "apps/sentinel/api/src/modules/user/controller.ts",
        "apps/sentinel/api/src/modules/user/service.ts",
        // Missing repo.ts
      ]),
    );

    expect(result.violations.filter((v) => v.rule === "match").length).toBe(1);
    expect(result.violations[0]?.message).toContain("repo.ts");
  });

  test("passes when all required files exist", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        match: [
          {
            pattern: "apps/*/api/src/modules/*",
            require: ["controller.ts", "service.ts", "repo.ts"],
          },
        ],
      },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "apps/sentinel/api/src/modules/user",
        "apps/sentinel/api/src/modules/user/controller.ts",
        "apps/sentinel/api/src/modules/user/service.ts",
        "apps/sentinel/api/src/modules/user/repo.ts",
      ]),
    );

    expect(result.violations.filter((v) => v.rule === "match").length).toBe(0);
  });

  test("validates strict mode - rejects unlisted files", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        match: [
          {
            pattern: "modules/*",
            require: ["controller.ts"],
            allow: ["service.ts"],
            strict: true,
          },
        ],
      },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "modules/user",
        "modules/user/controller.ts",
        "modules/user/service.ts",
        "modules/user/unauthorized.ts", // Not in require or allow
      ]),
    );

    const matchViolations = result.violations.filter((v) => v.rule === "match");
    expect(matchViolations.length).toBe(1);
    expect(matchViolations[0]?.path).toContain("unauthorized.ts");
  });

  test("validates forbidden files", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        match: [
          {
            pattern: "modules/*",
            forbid: ["*.test.ts", "*.spec.ts"],
          },
        ],
      },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "modules/user",
        "modules/user/controller.ts",
        "modules/user/controller.test.ts", // Forbidden
      ]),
    );

    expect(result.violations.filter((v) => v.rule === "match").length).toBe(1);
  });

  test("respects exclude patterns", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        match: [
          {
            pattern: "modules/*",
            exclude: ["modules/special"],
            require: ["controller.ts"],
          },
        ],
      },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "modules/user",
        "modules/user/controller.ts",
        "modules/special", // Excluded - no controller required
        "modules/special/custom.ts",
      ]),
    );

    expect(result.violations.filter((v) => v.rule === "match").length).toBe(0);
  });

  // =========================================================================
  // Case validation - directory name vs children
  // =========================================================================

  test("case validates matched directory name itself", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        match: [
          {
            pattern: "modules/*",
            case: "kebab", // Validates the directory name (e.g., "UserModule" should be "user-module")
          },
        ],
      },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "modules/UserModule", // PascalCase, not kebab
        "modules/UserModule/index.ts",
      ]),
    );

    const matchViolations = result.violations.filter((v) => v.rule === "match");
    expect(matchViolations.length).toBe(1);
    expect(matchViolations[0]?.path).toBe("modules/UserModule");
    expect(matchViolations[0]?.message).toContain("directory name");
  });

  test("childCase validates children names, not directory", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        match: [
          {
            pattern: "modules/*",
            childCase: "kebab", // Validates children, not the matched dir
          },
        ],
      },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "modules/user", // Directory name is fine
        "modules/user/MyController.ts", // PascalCase child, should fail
      ]),
    );

    const matchViolations = result.violations.filter((v) => v.rule === "match");
    expect(matchViolations.length).toBe(1);
    expect(matchViolations[0]?.path).toBe("modules/user/MyController.ts");
  });

  test("case and childCase can be used together", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        match: [
          {
            pattern: "components/*",
            case: "pascal", // Component directories should be PascalCase
            childCase: "kebab", // But files inside should be kebab-case
          },
        ],
      },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "components/button", // Should be Button (pascal)
        "components/button/Index.tsx", // Should be index.tsx (kebab)
      ]),
    );

    const matchViolations = result.violations.filter((v) => v.rule === "match");
    expect(matchViolations.length).toBe(2);
  });

  // =========================================================================
  // Edge cases - empty directories, root level, etc.
  // =========================================================================

  test("handles empty directories", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        match: [
          {
            pattern: "modules/*",
            require: ["index.ts"],
          },
        ],
      },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "modules/empty", // Directory exists but is empty
      ]),
    );

    const matchViolations = result.violations.filter((v) => v.rule === "match");
    expect(matchViolations.length).toBe(1);
    expect(matchViolations[0]?.message).toContain("index.ts");
  });

  test("handles root-level directory matches", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        match: [
          {
            pattern: "*",
            forbid: ["*.log"],
          },
        ],
      },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "src",
        "src/index.ts",
        "docs",
        "docs/readme.md",
      ]),
    );

    // Should match src and docs directories at root level
    expect(result.violations.filter((v) => v.rule === "match").length).toBe(0);
  });

  test("warns when pattern matches nothing", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        match: [
          {
            pattern: "this/path/does/not/exist/*",
            require: ["foo.ts"],
          },
        ],
      },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "src",
        "src/index.ts",
      ]),
    );

    const warnings = result.violations.filter(
      (v) => v.rule === "match" && v.severity === "warning"
    );
    expect(warnings.length).toBe(1);
    expect(warnings[0]?.message).toContain("did not match any directories");
  });

  // =========================================================================
  // Strict mode edge cases
  // =========================================================================

  test("strict mode with empty require/allow rejects all entries", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        match: [
          {
            pattern: "modules/*",
            strict: true,
            // No require, no allow - should reject everything
          },
        ],
      },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "modules/user",
        "modules/user/anything.ts",
        "modules/user/something-else.ts",
      ]),
    );

    const matchViolations = result.violations.filter((v) => v.rule === "match");
    expect(matchViolations.length).toBe(2); // Both files rejected
    expect(matchViolations[0]?.message).toContain("strict mode with no allowed patterns");
  });

  // =========================================================================
  // Overlapping rules
  // =========================================================================

  test("handles overlapping rules - both are applied", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        match: [
          {
            pattern: "modules/*",
            require: ["index.ts"],
          },
          {
            pattern: "modules/api-*",
            require: ["controller.ts"],
          },
        ],
      },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "modules/api-users",
        // Missing both index.ts (from first rule) and controller.ts (from second rule)
      ]),
    );

    const matchViolations = result.violations.filter((v) => v.rule === "match");
    expect(matchViolations.length).toBe(2);
    expect(matchViolations.some((v) => v.message.includes("index.ts"))).toBe(true);
    expect(matchViolations.some((v) => v.message.includes("controller.ts"))).toBe(true);
  });

  // =========================================================================
  // Deeply nested structures
  // =========================================================================

  test("handles deeply nested directory structures", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        match: [
          {
            pattern: "apps/*/packages/*/src/features/*",
            require: ["index.ts"],
          },
        ],
      },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "apps/web/packages/ui/src/features/auth",
        "apps/web/packages/ui/src/features/auth/index.ts",
        "apps/web/packages/ui/src/features/dashboard",
        // Missing index.ts in dashboard
      ]),
    );

    const matchViolations = result.violations.filter((v) => v.rule === "match");
    expect(matchViolations.length).toBe(1);
    expect(matchViolations[0]?.path).toBe("apps/web/packages/ui/src/features/dashboard");
  });

  // =========================================================================
  // Conflicting require and forbid
  // =========================================================================

  test("require and forbid with same entry - forbid wins", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        match: [
          {
            pattern: "modules/*",
            require: ["index.ts"],
            forbid: ["index.ts"], // Contradictory!
          },
        ],
      },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "modules/user",
        "modules/user/index.ts", // Required but also forbidden
      ]),
    );

    const matchViolations = result.violations.filter((v) => v.rule === "match");
    // Should have a forbidden violation (index.ts is present and forbidden)
    expect(matchViolations.some((v) => v.message.includes("forbidden"))).toBe(true);
  });

  // =========================================================================
  // Hidden files - should be EXEMPT from case validation
  // =========================================================================

  test("hidden files are exempt from case validation", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        match: [
          {
            pattern: "modules/*",
            childCase: "kebab",
          },
        ],
      },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "modules/user",
        "modules/user/.gitignore", // Hidden - exempt
        "modules/user/.DS_Store", // Hidden - exempt (even with uppercase)
        "modules/user/.env", // Hidden - exempt
        "modules/user/.eslintrc.json", // Hidden - exempt
        "modules/user/normal-file.ts", // Not hidden - validated
      ]),
    );

    // Hidden files should NOT produce violations
    const matchViolations = result.violations.filter((v) => v.rule === "match");
    expect(matchViolations.length).toBe(0);
  });

  test("hidden files with invalid non-hidden siblings still caught", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        match: [
          {
            pattern: "modules/*",
            childCase: "kebab",
          },
        ],
      },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "modules/user",
        "modules/user/.gitignore", // Hidden - exempt
        "modules/user/InvalidName.ts", // Not hidden - should fail
      ]),
    );

    const matchViolations = result.violations.filter((v) => v.rule === "match");
    expect(matchViolations.length).toBe(1);
    expect(matchViolations[0]?.path).toContain("InvalidName.ts");
  });

  // =========================================================================
  // Files without extensions
  // =========================================================================

  test("validates files without extensions", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        match: [
          {
            pattern: "bin/*",
            childCase: "kebab",
          },
        ],
      },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "bin/cli",
        "bin/cli/myScript", // No extension, camelCase - should fail
        "bin/cli/my-script", // No extension, kebab-case - should pass
      ]),
    );

    const matchViolations = result.violations.filter((v) => v.rule === "match");
    expect(matchViolations.length).toBe(1);
    expect(matchViolations[0]?.path).toContain("myScript");
  });

  // =========================================================================
  // Glob patterns in require/allow/forbid
  // =========================================================================

  test("supports glob patterns in require", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        match: [
          {
            pattern: "modules/*",
            require: ["*.ts"], // At least one .ts file required
          },
        ],
      },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "modules/styles",
        "modules/styles/main.css", // No .ts file
      ]),
    );

    const matchViolations = result.violations.filter((v) => v.rule === "match");
    expect(matchViolations.length).toBe(1);
  });

  test("supports glob patterns in allow with strict", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        match: [
          {
            pattern: "modules/*",
            allow: ["*.ts", "*.tsx"],
            strict: true,
          },
        ],
      },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "modules/user",
        "modules/user/index.ts", // Allowed
        "modules/user/styles.css", // Not allowed
      ]),
    );

    const matchViolations = result.violations.filter((v) => v.rule === "match");
    expect(matchViolations.length).toBe(1);
    expect(matchViolations[0]?.path).toContain("styles.css");
  });

  // =========================================================================
  // Suggestions in violations
  // =========================================================================

  test("provides case suggestions in violations", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        match: [
          {
            pattern: "modules/*",
            case: "kebab",
          },
        ],
      },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "modules/UserProfile", // PascalCase - should suggest user-profile
      ]),
    );

    const matchViolations = result.violations.filter((v) => v.rule === "match");
    expect(matchViolations.length).toBe(1);
    expect(matchViolations[0]?.suggestions).toBeDefined();
    expect(matchViolations[0]?.suggestions?.[0]).toBe("user-profile");
  });

  // =========================================================================
  // Empty pattern handling
  // =========================================================================

  test("warns on empty pattern", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        match: [
          {
            pattern: "",
            require: ["index.ts"],
          },
        ],
      },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "src",
        "src/index.ts",
      ]),
    );

    const warnings = result.violations.filter(
      (v) => v.rule === "match" && v.severity === "warning"
    );
    expect(warnings.length).toBe(1);
    expect(warnings[0]?.message).toContain("empty pattern");
  });

  test("warns on whitespace-only pattern", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        match: [
          {
            pattern: "   ",
            require: ["index.ts"],
          },
        ],
      },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "src",
        "src/index.ts",
      ]),
    );

    const warnings = result.violations.filter(
      (v) => v.rule === "match" && v.severity === "warning"
    );
    expect(warnings.length).toBe(1);
    expect(warnings[0]?.message).toContain("empty pattern");
  });

  // =========================================================================
  // Duplicate violation prevention
  // =========================================================================

  test("deduplicates violations from overlapping rules", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        match: [
          {
            pattern: "modules/*",
            forbid: ["temp.ts"],
          },
          {
            pattern: "modules/user",
            forbid: ["temp.ts"],
          },
        ],
      },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "modules/user",
        "modules/user/temp.ts", // Both rules match, but should only get ONE violation
      ]),
    );

    const matchViolations = result.violations.filter((v) => v.rule === "match");
    expect(matchViolations.length).toBe(1); // Deduplicated!
  });

  // =========================================================================
  // Very deep nesting (stack overflow protection)
  // =========================================================================

  test("handles very deep directory nesting", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        match: [
          {
            pattern: "a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t",
            require: ["index.ts"],
          },
        ],
      },
    };

    // Create a 20-level deep path
    const deepPath = "a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t";
    const result = await runCheck(
      config,
      makeFiles([
        deepPath,
        `${deepPath}/index.ts`,
      ]),
    );

    // Should not throw stack overflow
    const matchViolations = result.violations.filter((v) => v.rule === "match");
    expect(matchViolations.length).toBe(0);
  });

  // =========================================================================
  // Paths with spaces
  // =========================================================================

  test("handles paths with spaces", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        match: [
          {
            pattern: "my modules/*",
            require: ["index.ts"],
          },
        ],
      },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "my modules/user",
        "my modules/user/index.ts",
      ]),
    );

    const matchViolations = result.violations.filter((v) => v.rule === "match");
    expect(matchViolations.length).toBe(0);
  });

  // =========================================================================
  // Special characters in paths
  // =========================================================================

  test("handles special characters in directory names", async () => {
    const config: RepoLintConfig = {
      mode: "strict",
      rules: {
        match: [
          {
            pattern: "modules/*",
            require: ["index.ts"],
          },
        ],
      },
    };

    const result = await runCheck(
      config,
      makeFiles([
        "modules/@scope",
        "modules/@scope/index.ts",
        "modules/module-v2.0",
        "modules/module-v2.0/index.ts",
      ]),
    );

    const matchViolations = result.violations.filter((v) => v.rule === "match");
    expect(matchViolations.length).toBe(0);
  });
});
