import { describe, expect, test } from "bun:test";
import { Effect } from "effect";
import type {
  CaseStyle,
  Severity,
  Mode,
  LayoutNode,
  RepoLintConfig,
  Violation,
  CheckResult,
  FileEntry,
} from "../src/types.js";

describe("types", () => {
  test("CaseStyle type includes expected values", () => {
    const validateStyles = Effect.sync(() => {
      const styles: CaseStyle[] = ["kebab", "snake", "camel", "pascal", "any"];
      return styles.length;
    });

    const result = Effect.runSync(validateStyles);
    expect(result).toBe(5);
  });

  test("Severity type includes expected values", () => {
    const validateSeverities = Effect.sync(() => {
      const severities: Severity[] = ["error", "warning"];
      return severities;
    });

    const result = Effect.runSync(validateSeverities);
    expect(result).toHaveLength(2);
  });

  test("Mode type includes expected values", () => {
    const validateModes = Effect.sync(() => {
      const modes: Mode[] = ["strict", "warn"];
      return modes;
    });

    const result = Effect.runSync(validateModes);
    expect(result).toHaveLength(2);
  });

  test("LayoutNode can represent file", () => {
    const createNode = Effect.sync((): LayoutNode => ({ type: "file", pattern: "*.ts" }));
    const node = Effect.runSync(createNode);
    expect(node.type).toBe("file");
  });

  test("LayoutNode can represent dir with children", () => {
    const createNode = Effect.sync((): LayoutNode => ({
      type: "dir",
      children: { "index.ts": { type: "file" } },
    }));

    const node = Effect.runSync(createNode);
    expect(node.type).toBe("dir");
    expect(node.children?.["index.ts"]).toBeDefined();
  });

  test("LayoutNode can represent recursive", () => {
    const createNode = Effect.sync((): LayoutNode => ({
      type: "recursive",
      case: "kebab",
      child: { type: "dir", children: {} },
    }));

    const node = Effect.runSync(createNode);
    expect(node.type).toBe("recursive");
    expect(node.case).toBe("kebab");
  });

  test("RepoLintConfig has correct shape", () => {
    const createConfig = Effect.sync((): RepoLintConfig => ({
      mode: "strict",
      ignore: ["node_modules"],
      rules: { forbidPaths: ["**/utils/**"], forbidNames: ["temp"] },
    }));

    const config = Effect.runSync(createConfig);
    expect(config.mode).toBe("strict");
  });

  test("Violation has correct shape", () => {
    const createViolation = Effect.sync((): Violation => ({
      path: "src/file.ts",
      rule: "naming",
      message: "invalid name",
      severity: "error",
    }));

    const violation = Effect.runSync(createViolation);
    expect(violation.severity).toBe("error");
  });

  test("CheckResult has correct shape", () => {
    const createResult = Effect.sync((): CheckResult => ({
      violations: [],
      summary: { total: 0, errors: 0, warnings: 0, filesChecked: 10, duration: 5 },
    }));

    const result = Effect.runSync(createResult);
    expect(result.summary.filesChecked).toBe(10);
  });

  test("FileEntry has correct shape", () => {
    const createEntry = Effect.sync((): FileEntry => ({
      path: "/full/path.ts",
      relativePath: "path.ts",
      isDirectory: false,
      depth: 1,
    }));

    const entry = Effect.runSync(createEntry);
    expect(entry.isDirectory).toBe(false);
  });
});
