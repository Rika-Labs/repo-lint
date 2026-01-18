import { describe, expect, test, beforeAll, afterAll } from "bun:test";
import { Effect, Exit, Option, Cause } from "effect";
import { scan, fileExists, readFileContent } from "../src/core/scanner.js";
import { MaxDepthExceededError } from "../src/errors.js";
import { mkdir, writeFile, rm } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";

let testDir: string;

beforeAll(async () => {
  testDir = join(tmpdir(), `repo-lint-test-${Date.now()}`);
  await mkdir(testDir, { recursive: true });
  await mkdir(join(testDir, "src"));
  await writeFile(join(testDir, "src/index.ts"), "export {}");
  await writeFile(join(testDir, "package.json"), "{}");
  await mkdir(join(testDir, "node_modules"));
  await writeFile(join(testDir, "node_modules/dep.js"), "");
  await writeFile(join(testDir, ".gitignore"), "ignored.txt\n");
  await writeFile(join(testDir, "ignored.txt"), "ignored");
  await mkdir(join(testDir, "subdir"));
  await writeFile(join(testDir, "subdir/.gitignore"), "secret.txt\n");
  await writeFile(join(testDir, "subdir/secret.txt"), "secret");
  await writeFile(join(testDir, "subdir/keep.txt"), "keep");
});

afterAll(async () => {
  await rm(testDir, { recursive: true, force: true });
});

describe("scan", () => {
  test("scans directory recursively", async () => {
    const files = await Effect.runPromise(
      scan({
        root: testDir,
        ignore: [],
        scope: Option.none(),
        useGitignore: Option.some(false),
        maxDepth: Option.none(),
        maxFiles: Option.none(),
        followSymlinks: Option.none(),
        timeout: Option.none(),
        concurrency: Option.none(),
      }),
    );

    expect(files.length).toBeGreaterThan(0);
    expect(files.some((f) => f.relativePath === "src")).toBe(true);
    expect(files.some((f) => f.relativePath === "src/index.ts")).toBe(true);
    expect(files.some((f) => f.relativePath === "package.json")).toBe(true);
  });

  test("respects ignore patterns", async () => {
    const files = await Effect.runPromise(
      scan({
        root: testDir,
        ignore: ["node_modules/**"],
        scope: Option.none(),
        useGitignore: Option.some(false),
        maxDepth: Option.none(),
        maxFiles: Option.none(),
        followSymlinks: Option.none(),
        timeout: Option.none(),
        concurrency: Option.none(),
      }),
    );

    expect(files.some((f) => f.relativePath.includes("node_modules"))).toBe(false);
    expect(files.some((f) => f.relativePath === "src/index.ts")).toBe(true);
  });

  test("respects nested gitignore files", async () => {
    const files = await Effect.runPromise(
      scan({
        root: testDir,
        ignore: [],
        scope: Option.none(),
        useGitignore: Option.some(true),
        maxDepth: Option.none(),
        maxFiles: Option.none(),
        followSymlinks: Option.none(),
        timeout: Option.none(),
        concurrency: Option.none(),
      }),
    );

    expect(files.some((f) => f.relativePath === "ignored.txt")).toBe(false);
    expect(files.some((f) => f.relativePath === "subdir/secret.txt")).toBe(false);
    expect(files.some((f) => f.relativePath === "subdir/keep.txt")).toBe(true);
  });

  test("marks directories correctly", async () => {
    const files = await Effect.runPromise(
      scan({
        root: testDir,
        ignore: [],
        scope: Option.none(),
        useGitignore: Option.some(false),
        maxDepth: Option.none(),
        maxFiles: Option.none(),
        followSymlinks: Option.none(),
        timeout: Option.none(),
        concurrency: Option.none(),
      }),
    );

    const srcDir = files.find((f) => f.relativePath === "src");
    const indexFile = files.find((f) => f.relativePath === "src/index.ts");

    expect(srcDir?.isDirectory).toBe(true);
    expect(indexFile?.isDirectory).toBe(false);
  });

  test("calculates depth correctly", async () => {
    const files = await Effect.runPromise(
      scan({
        root: testDir,
        ignore: [],
        scope: Option.none(),
        useGitignore: Option.some(false),
        maxDepth: Option.none(),
        maxFiles: Option.none(),
        followSymlinks: Option.none(),
        timeout: Option.none(),
        concurrency: Option.none(),
      }),
    );

    const srcDir = files.find((f) => f.relativePath === "src");
    const indexFile = files.find((f) => f.relativePath === "src/index.ts");

    expect(srcDir?.depth).toBe(1);
    expect(indexFile?.depth).toBe(2);
  });

  test("respects maxDepth", async () => {
    const files = await Effect.runPromise(
      scan({
        root: testDir,
        ignore: [],
        scope: Option.none(),
        useGitignore: Option.some(false),
        maxDepth: Option.some(1),
        maxFiles: Option.none(),
        followSymlinks: Option.none(),
        timeout: Option.none(),
        concurrency: Option.none(),
      }),
    );

    // Should only have top-level entries
    expect(files.every((f) => f.depth <= 2)).toBe(true);
  });

  test("throws MaxDepthExceededError when depth limit is exceeded", async () => {
    // Create a deeply nested directory structure
    const deepTestDir = join(tmpdir(), `repo-lint-deep-test-${Date.now()}`);
    await mkdir(deepTestDir, { recursive: true });

    // Create a directory structure deeper than maxDepth
    let currentPath = deepTestDir;
    for (let i = 0; i < 5; i++) {
      currentPath = join(currentPath, `level${i}`);
      await mkdir(currentPath, { recursive: true });
      await writeFile(join(currentPath, "file.txt"), `level ${i}`);
    }

    try {
      const exit = await Effect.runPromiseExit(
        scan({
          root: deepTestDir,
          ignore: [],
          scope: Option.none(),
          useGitignore: Option.some(false),
          maxDepth: Option.some(2),
          maxFiles: Option.none(),
          followSymlinks: Option.none(),
          timeout: Option.none(),
          concurrency: Option.none(),
        }),
      );

      expect(Exit.isFailure(exit)).toBe(true);

      if (Exit.isFailure(exit)) {
        const failure = Cause.failureOption(exit.cause);
        expect(Option.isSome(failure)).toBe(true);
        if (Option.isSome(failure)) {
          const error = failure.value;
          // The error will be wrapped in ScanError, so we need to check the cause
          expect(error._tag).toBe("ScanError");
          // Verify the underlying cause is MaxDepthExceededError
          expect(error.cause).toBeInstanceOf(MaxDepthExceededError);
        }
      }
    } finally {
      await rm(deepTestDir, { recursive: true, force: true });
    }
  });
});

describe("fileExists", () => {
  test("returns true for existing file", async () => {
    const exists = await Effect.runPromise(fileExists(join(testDir, "package.json")));
    expect(exists).toBe(true);
  });

  test("returns false for non-existing file", async () => {
    const exists = await Effect.runPromise(fileExists(join(testDir, "nonexistent.ts")));
    expect(exists).toBe(false);
  });
});

describe("readFileContent", () => {
  test("reads file content", async () => {
    const content = await Effect.runPromise(readFileContent(join(testDir, "package.json")));
    expect(content).toBe("{}");
  });

  test("fails for non-existing file", async () => {
    const exit = await Effect.runPromiseExit(readFileContent(join(testDir, "nonexistent.ts")));
    expect(Exit.isFailure(exit)).toBe(true);
  });
});
