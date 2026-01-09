import { describe, expect, test, beforeEach, afterEach } from "bun:test";
import { Effect, Exit, Option } from "effect";
import { mkdir, writeFile, rm } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { runCheck, type ScanOverrides } from "../src/commands/check.js";
import { runInspect } from "../src/commands/inspect.js";
import { getCacheStats, clearCache } from "../src/cache/index.js";

let testDir: string;
let originalCwd: string;

const noOverrides: ScanOverrides = {
  maxDepth: Option.none(),
  maxFiles: Option.none(),
  timeoutMs: Option.none(),
  concurrency: Option.none(),
  followSymlinks: Option.none(),
  useGitignore: Option.none(),
};

beforeEach(async () => {
  originalCwd = process.cwd();
  testDir = join(tmpdir(), `repo-lint-commands-test-${Date.now()}`);
  await mkdir(testDir, { recursive: true });
  await writeFile(
    join(testDir, ".repo-lint.yaml"),
    `mode: strict
layout:
  type: dir
  children:
    "a.txt": { required: true }
    ".repo-lint.yaml": { optional: true }
`,
  );
  await writeFile(join(testDir, "a.txt"), "ok");
  process.chdir(testDir);
});

afterEach(async () => {
  process.chdir(originalCwd);
  await rm(testDir, { recursive: true, force: true });
});

describe("runCheck", () => {
  test("returns result and writes cache", async () => {
    const result = await Effect.runPromise(
      runCheck({
        scope: Option.none(),
        format: "json",
        configPath: Option.none(),
        noCache: Option.none(),
        scanOverrides: noOverrides,
      }),
    );

    expect(result.summary.errors).toBe(0);
    const stats = await Effect.runPromise(getCacheStats(testDir));
    expect(Option.isSome(stats)).toBe(true);
  });

  test("fails on path traversal scope", async () => {
    const exit = await Effect.runPromiseExit(
      runCheck({
        scope: Option.some("../"),
        format: "json",
        configPath: Option.none(),
        noCache: Option.some(true),
        scanOverrides: noOverrides,
      }),
    );

    expect(Exit.isFailure(exit)).toBe(true);
  });

  test("no-cache skips cache updates", async () => {
    await Effect.runPromise(clearCache(testDir));

    await Effect.runPromise(
      runCheck({
        scope: Option.none(),
        format: "json",
        configPath: Option.none(),
        noCache: Option.some(true),
        scanOverrides: noOverrides,
      }),
    );

    const stats = await Effect.runPromise(getCacheStats(testDir));
    expect(Option.isNone(stats)).toBe(true);
  });

  test("scan overrides affect results", async () => {
    const customConfig = join(testDir, "override.yaml");
    await writeFile(
      customConfig,
      `mode: strict
layout:
  type: dir
  children:
    "nested":
      type: dir
      children:
        "file.txt": { required: true }
    "override.yaml": { optional: true }
`,
    );
    await mkdir(join(testDir, "nested"), { recursive: true });
    await writeFile(join(testDir, "nested/file.txt"), "ok");

    const result = await Effect.runPromise(
      runCheck({
        scope: Option.none(),
        format: "json",
        configPath: Option.some(customConfig),
        noCache: Option.some(true),
        scanOverrides: {
          ...noOverrides,
          maxDepth: Option.some(1),
        },
      }),
    );

    expect(result.summary.errors).toBeGreaterThan(0);
  });
});

describe("runInspect", () => {
  test("prints layout without error", async () => {
    const result = await Effect.runPromise(
      runInspect({
        type: "layout",
        arg: Option.none(),
        configPath: Option.none(),
      }),
    );
    expect(result).toBeUndefined();
  });

  test("prints rule info without error", async () => {
    const result = await Effect.runPromise(
      runInspect({
        type: "rule",
        arg: Option.some("forbidNames"),
        configPath: Option.none(),
      }),
    );
    expect(result).toBeUndefined();
  });
});
