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

describe("cache race condition protection", () => {
  test("concurrent writes do not corrupt cache", async () => {
    const files1 = makeFiles(["src/a.ts"]);
    const files2 = makeFiles(["src/b.ts"]);
    const fileHash1 = computeFileHash(files1);
    const fileHash2 = computeFileHash(files2);
    const configContent = "mode: strict";

    const result1: CheckResult = {
      violations: [{ message: "test1", severity: "error", path: "a.ts" }],
      summary: { total: 1, errors: 1, warnings: 0, filesChecked: 1, duration: 100 },
    };

    const result2: CheckResult = {
      violations: [{ message: "test2", severity: "error", path: "b.ts" }],
      summary: { total: 1, errors: 1, warnings: 0, filesChecked: 1, duration: 200 },
    };

    // Run two concurrent writes
    await Promise.all([
      Effect.runPromise(writeCache(testDir, configContent, fileHash1, files1.length, result1)),
      Effect.runPromise(writeCache(testDir, configContent, fileHash2, files2.length, result2)),
    ]);

    // Cache should contain one of the results (not corrupted)
    const cached1 = await Effect.runPromise(readCache(testDir, configContent, fileHash1));
    const cached2 = await Effect.runPromise(readCache(testDir, configContent, fileHash2));

    // Either result1 or result2 should be cached (both wrote successfully, one overwrote the other)
    // The important thing is the cache is not corrupted
    const hasValidCache = Option.isSome(cached1) || Option.isSome(cached2);
    expect(hasValidCache).toBe(true);
  });

  test("concurrent read and write operations", async () => {
    const files = makeFiles(["src/a.ts", "src/b.ts"]);
    const fileHash = computeFileHash(files);
    const configContent = "mode: strict";

    // Write initial cache
    await Effect.runPromise(writeCache(testDir, configContent, fileHash, files.length, emptyResult));

    // Run concurrent reads and a write
    const operations = [
      Effect.runPromise(readCache(testDir, configContent, fileHash)),
      Effect.runPromise(readCache(testDir, configContent, fileHash)),
      Effect.runPromise(
        writeCache(testDir, configContent, fileHash, files.length, {
          violations: [{ message: "test", severity: "error", path: "a.ts" }],
          summary: { total: 1, errors: 1, warnings: 0, filesChecked: 1, duration: 100 },
        }),
      ),
      Effect.runPromise(readCache(testDir, configContent, fileHash)),
    ];

    // All operations should complete without errors
    const results = await Promise.all(operations);

    // All reads should return valid Options (Some or None)
    expect(results[0]).toBeDefined();
    expect(results[1]).toBeDefined();
    expect(results[3]).toBeDefined();
  });

  test("handles stale lock files", async () => {
    const files = makeFiles(["src/a.ts"]);
    const fileHash = computeFileHash(files);
    const configContent = "mode: strict";

    // Create a fake stale lock file
    const { mkdir, writeFile } = await import("node:fs/promises");
    const lockPath = join(testDir, ".repo-lint-cache", ".lock");
    await mkdir(join(testDir, ".repo-lint-cache"), { recursive: true });
    await writeFile(lockPath, "99999", { flag: "w" });

    // Set the lock file's mtime to be very old
    const { utimes } = await import("node:fs/promises");
    const oldTime = Date.now() / 1000 - 10000; // 10000 seconds ago
    await utimes(lockPath, oldTime, oldTime);

    // Write should detect stale lock and succeed
    await Effect.runPromise(writeCache(testDir, configContent, fileHash, files.length, emptyResult));

    // Verify cache was written
    const cached = await Effect.runPromise(readCache(testDir, configContent, fileHash));
    expect(Option.isSome(cached)).toBe(true);
  });
});
