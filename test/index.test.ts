import { describe, expect, test } from "bun:test";
import { Effect } from "effect";
import * as RepoLint from "../src/index.js";

describe("index exports", () => {
  test("exports type utilities", async () => {
    const program = Effect.gen(function* () {
      expect(RepoLint.validateCase).toBeDefined();
      expect(RepoLint.suggestCase).toBeDefined();
      expect(RepoLint.getCaseName).toBeDefined();
      expect(RepoLint.validateCaseEffect).toBeDefined();

      const isValid = yield* RepoLint.validateCaseEffect("my-component", "kebab");
      return isValid;
    });

    const result = await Effect.runPromise(program);
    expect(result).toBe(true);
  });

  test("exports matcher utilities", async () => {
    const program = Effect.gen(function* () {
      expect(RepoLint.matches).toBeDefined();
      expect(RepoLint.matchesAny).toBeDefined();
      expect(RepoLint.createMatcher).toBeDefined();
      expect(RepoLint.matchesEffect).toBeDefined();
      expect(RepoLint.matchesAnyEffect).toBeDefined();

      const isMatch = yield* RepoLint.matchesEffect("src/index.ts", "**/*.ts");
      return isMatch;
    });

    const result = await Effect.runPromise(program);
    expect(result).toBe(true);
  });

  test("exports scanner functions", () => {
    expect(RepoLint.scan).toBeDefined();
    expect(RepoLint.scanWorkspaces).toBeDefined();
    expect(RepoLint.readFileContent).toBeDefined();
    expect(RepoLint.fileExists).toBeDefined();
  });

  test("exports config functions", () => {
    expect(RepoLint.findConfig).toBeDefined();
    expect(RepoLint.loadConfig).toBeDefined();
    expect(RepoLint.loadConfigFromRoot).toBeDefined();
    expect(RepoLint.findWorkspaceConfigs).toBeDefined();
  });

  test("exports checker", async () => {
    expect(RepoLint.check).toBeDefined();

    const result = await Effect.runPromise(RepoLint.check({ mode: "strict" }, []));

    expect(result.violations).toEqual([]);
    expect(result.summary.filesChecked).toBe(0);
  });

  test("exports formatters", async () => {
    expect(RepoLint.format).toBeDefined();
    expect(RepoLint.formatConsole).toBeDefined();
    expect(RepoLint.formatJson).toBeDefined();
    expect(RepoLint.formatSarif).toBeDefined();
    expect(RepoLint.formatEffect).toBeDefined();

    const checkResult: RepoLint.CheckResult = {
      violations: [],
      summary: { total: 0, errors: 0, warnings: 0, filesChecked: 0, duration: 0 },
    };

    const output = await Effect.runPromise(RepoLint.formatEffect(checkResult, "json"));
    const parsed = JSON.parse(output) as RepoLint.CheckResult;
    expect(parsed.violations).toEqual([]);
  });

  test("exports errors", () => {
    expect(RepoLint.ConfigNotFoundError).toBeDefined();
    expect(RepoLint.ConfigParseError).toBeDefined();
    expect(RepoLint.ConfigValidationError).toBeDefined();
    expect(RepoLint.FileSystemError).toBeDefined();
    expect(RepoLint.ScanError).toBeDefined();
  });

  test("exports presets", () => {
    expect(RepoLint.nextjsPreset).toBeDefined();

    const preset = RepoLint.nextjsPreset();
    expect(preset.mode).toBe("strict");
  });
});
