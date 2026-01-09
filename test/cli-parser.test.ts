import { describe, expect, test } from "bun:test";
import { Option } from "effect";
import { parseArgs } from "../src/cli/parser.js";

describe("cli parser", () => {
  test("parses scan override flags", () => {
    const args = parseArgs([
      "check",
      "--max-depth",
      "5",
      "--max-files",
      "100",
      "--timeout-ms",
      "2000",
      "--concurrency",
      "3",
      "--follow-symlinks",
      "--no-gitignore",
    ]);

    expect(Option.isSome(args.scanOverrides.maxDepth)).toBe(true);
    if (Option.isSome(args.scanOverrides.maxDepth)) {
      expect(args.scanOverrides.maxDepth.value).toBe(5);
    }
    expect(Option.isSome(args.scanOverrides.maxFiles)).toBe(true);
    if (Option.isSome(args.scanOverrides.maxFiles)) {
      expect(args.scanOverrides.maxFiles.value).toBe(100);
    }
    expect(Option.isSome(args.scanOverrides.timeoutMs)).toBe(true);
    if (Option.isSome(args.scanOverrides.timeoutMs)) {
      expect(args.scanOverrides.timeoutMs.value).toBe(2000);
    }
    expect(Option.isSome(args.scanOverrides.concurrency)).toBe(true);
    if (Option.isSome(args.scanOverrides.concurrency)) {
      expect(args.scanOverrides.concurrency.value).toBe(3);
    }
    expect(Option.isSome(args.scanOverrides.followSymlinks)).toBe(true);
    if (Option.isSome(args.scanOverrides.followSymlinks)) {
      expect(args.scanOverrides.followSymlinks.value).toBe(true);
    }
    expect(Option.isSome(args.scanOverrides.useGitignore)).toBe(true);
    if (Option.isSome(args.scanOverrides.useGitignore)) {
      expect(args.scanOverrides.useGitignore.value).toBe(false);
    }
  });

  test("reports errors on invalid numbers", () => {
    const args = parseArgs(["check", "--max-depth", "abc", "--concurrency", "-1"]);
    expect(args.errors.length).toBeGreaterThan(0);
  });

  test("reports missing value for scope", () => {
    const args = parseArgs(["check", "--scope"]);
    expect(args.errors.length).toBeGreaterThan(0);
  });
});
