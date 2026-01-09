import { describe, expect, test } from "bun:test";
import { format } from "../src/output/formatters.js";
import type { CheckResult } from "../src/types/index.js";

const emptyResult: CheckResult = {
  violations: [],
  summary: { total: 0, errors: 0, warnings: 0, filesChecked: 10, duration: 5 },
};

const resultWithErrors: CheckResult = {
  violations: [
    { path: "src/bad.ts", rule: "naming", message: "expected kebab-case", severity: "error", got: "Bad" },
    { path: "src/temp.ts", rule: "forbidNames", message: "forbidden name", severity: "warning" },
  ],
  summary: { total: 2, errors: 1, warnings: 1, filesChecked: 20, duration: 10 },
};

describe("formatConsole", () => {
  test("formats empty result", () => {
    const output = format(emptyResult, "console");
    expect(output).toContain("No issues found");
    expect(output).toContain("10 files");
  });

  test("formats result with errors", () => {
    const output = format(resultWithErrors, "console");
    expect(output).toContain("error");
    expect(output).toContain("naming");
    expect(output).toContain("src/bad.ts");
  });
});

describe("formatJson", () => {
  test("outputs valid JSON", () => {
    const output = format(resultWithErrors, "json");
    const parsed = JSON.parse(output) as CheckResult;
    expect(parsed.violations).toHaveLength(2);
    expect(parsed.summary.total).toBe(2);
  });
});

describe("formatSarif", () => {
  test("outputs valid SARIF", () => {
    const output = format(resultWithErrors, "sarif");
    const parsed = JSON.parse(output) as { version: string; runs: Array<{ results: unknown[] }> };
    expect(parsed.version).toBe("2.1.0");
    expect(parsed.runs[0]?.results).toHaveLength(2);
  });

  test("includes rule definitions", () => {
    const output = format(resultWithErrors, "sarif");
    const parsed = JSON.parse(output) as { runs: Array<{ tool: { driver: { rules: unknown[] } } }> };
    expect(parsed.runs[0]?.tool.driver.rules.length).toBeGreaterThan(0);
  });
});

describe("format", () => {
  test("all formats work", () => {
    const consoleOutput = format(emptyResult, "console");
    const jsonOutput = format(emptyResult, "json");
    const sarifOutput = format(emptyResult, "sarif");

    expect(consoleOutput).toContain("No issues");
    const jsonResult = JSON.parse(jsonOutput) as CheckResult;
    expect(jsonResult.summary.total).toBe(0);
    const sarifResult = JSON.parse(sarifOutput) as { version: string };
    expect(sarifResult.version).toBe("2.1.0");
  });
});
