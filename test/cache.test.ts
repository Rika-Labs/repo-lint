import { describe, expect, test, beforeAll, afterAll } from "bun:test";
import { Effect, Option } from "effect";
import { mkdir, rm } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import {
  computeFileHash,
  readCache,
  writeCache,
  clearCache,
  getCacheStats,
} from "../src/cache/index.js";
import type { FileEntry, CheckResult } from "../src/types/index.js";

let testDir: string;

const makeFiles = (paths: readonly string[]): readonly FileEntry[] =>
  paths.map((p) => ({
    path: `/test/${p}`,
    relativePath: p,
    isDirectory: !p.includes("."),
    isSymlink: false,
    depth: p.split("/").length,
  }));

const emptyResult: CheckResult = {
  violations: [],
  summary: { total: 0, errors: 0, warnings: 0, filesChecked: 0, duration: 0 },
};

beforeAll(async () => {
  testDir = join(tmpdir(), `repo-lint-cache-test-${Date.now()}`);
  await mkdir(testDir, { recursive: true });
});

afterAll(async () => {
  await rm(testDir, { recursive: true, force: true });
});

describe("computeFileHash", () => {
  test("is deterministic regardless of order", () => {
    const filesA = [...makeFiles(["src/a.ts", "src/b.ts", "src"])].reverse();
    const filesB = makeFiles(["src", "src/a.ts", "src/b.ts"]);

    expect(computeFileHash(filesA)).toBe(computeFileHash(filesB));
  });
});

describe("cache read/write", () => {
  test("writes and reads cache entries", async () => {
    const files = makeFiles(["src/a.ts", "src/b.ts"]);
    const fileHash = computeFileHash(files);
    const configContent = "mode: strict";

    await Effect.runPromise(writeCache(testDir, configContent, fileHash, files.length, emptyResult));

    const cached = await Effect.runPromise(readCache(testDir, configContent, fileHash));
    expect(Option.isSome(cached)).toBe(true);
    if (Option.isSome(cached)) {
      expect(cached.value.result.summary.total).toBe(0);
      expect(cached.value.filesCount).toBe(files.length);
    }
  });

  test("invalidates cache when file hash changes", async () => {
    const files = makeFiles(["src/a.ts"]);
    const fileHash = computeFileHash(files);
    const configContent = "mode: strict";

    await Effect.runPromise(writeCache(testDir, configContent, fileHash, files.length, emptyResult));

    const differentHash = computeFileHash(makeFiles(["src/b.ts"]));
    const cached = await Effect.runPromise(readCache(testDir, configContent, differentHash));
    expect(Option.isNone(cached)).toBe(true);
  });

  test("invalidates cache when config changes", async () => {
    const files = makeFiles(["src/a.ts"]);
    const fileHash = computeFileHash(files);

    await Effect.runPromise(writeCache(testDir, "mode: strict", fileHash, files.length, emptyResult));

    const cached = await Effect.runPromise(readCache(testDir, "mode: warn", fileHash));
    expect(Option.isNone(cached)).toBe(true);
  });
});

describe("cache maintenance", () => {
  test("getCacheStats returns stats when cache exists", async () => {
    const files = makeFiles(["src/a.ts"]);
    const fileHash = computeFileHash(files);

    await Effect.runPromise(writeCache(testDir, "mode: strict", fileHash, files.length, emptyResult));

    const stats = await Effect.runPromise(getCacheStats(testDir));
    expect(Option.isSome(stats)).toBe(true);
    if (Option.isSome(stats)) {
      expect(stats.value.size).toBeGreaterThan(0);
      expect(stats.value.filesCount).toBe(files.length);
    }
  });

  test("clearCache removes cached data", async () => {
    await Effect.runPromise(clearCache(testDir));
    const stats = await Effect.runPromise(getCacheStats(testDir));
    expect(Option.isNone(stats)).toBe(true);
  });
});
