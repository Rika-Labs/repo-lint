import { describe, expect, test, beforeAll, afterAll } from "bun:test";
import { Effect, Option, Exit } from "effect";
import { findConfig, loadConfig, loadConfigFromRoot } from "../src/config.js";
import { mkdir, writeFile, rm } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";

let testDir: string;

beforeAll(async () => {
  testDir = join(tmpdir(), `repo-lint-config-test-${Date.now()}`);
  await mkdir(testDir, { recursive: true });
});

afterAll(async () => {
  await rm(testDir, { recursive: true, force: true });
});

describe("findConfig", () => {
  test("returns none when no config exists", async () => {
    const emptyDir = join(testDir, "empty");
    await mkdir(emptyDir, { recursive: true });

    const result = await Effect.runPromise(findConfig(emptyDir));
    expect(Option.isNone(result)).toBe(true);
  });

  test("finds repo-lint.config.yaml", async () => {
    const configDir = join(testDir, "with-config");
    await mkdir(configDir, { recursive: true });
    await writeFile(join(configDir, "repo-lint.config.yaml"), "mode: strict\n");

    const result = await Effect.runPromise(findConfig(configDir));
    expect(Option.isSome(result)).toBe(true);
    if (Option.isSome(result)) {
      expect(result.value).toContain("repo-lint.config.yaml");
    }
  });

  test("finds .repo-lint.yaml", async () => {
    const configDir = join(testDir, "with-dot-config");
    await mkdir(configDir, { recursive: true });
    await writeFile(join(configDir, ".repo-lint.yaml"), "mode: warn\n");

    const result = await Effect.runPromise(findConfig(configDir));
    expect(Option.isSome(result)).toBe(true);
  });
});

describe("loadConfig", () => {
  test("loads valid YAML config", async () => {
    const configDir = join(testDir, "load-test");
    await mkdir(configDir, { recursive: true });
    const configPath = join(configDir, "repo-lint.config.yaml");
    await writeFile(
      configPath,
      `mode: strict
ignore:
  - node_modules
rules:
  forbidNames:
    - temp
`
    );

    const config = await Effect.runPromise(loadConfig(configPath));
    expect(config.mode).toBe("strict");
    expect(config.ignore).toContain("node_modules");
    expect(config.rules?.forbidNames).toContain("temp");
  });

  test("fails on invalid YAML", async () => {
    const configDir = join(testDir, "invalid-yaml");
    await mkdir(configDir, { recursive: true });
    const configPath = join(configDir, "repo-lint.config.yaml");
    await writeFile(configPath, "invalid: yaml: content:");

    const exit = await Effect.runPromiseExit(loadConfig(configPath));
    expect(Exit.isFailure(exit)).toBe(true);
  });
});

describe("loadConfigFromRoot", () => {
  test("loads config from root directory", async () => {
    const configDir = join(testDir, "root-test");
    await mkdir(configDir, { recursive: true });
    await writeFile(join(configDir, "repo-lint.config.yaml"), "mode: warn\n");

    const config = await Effect.runPromise(loadConfigFromRoot(configDir));
    expect(config.mode).toBe("warn");
  });

  test("fails when no config exists", async () => {
    const emptyDir = join(testDir, "no-config");
    await mkdir(emptyDir, { recursive: true });

    const exit = await Effect.runPromiseExit(loadConfigFromRoot(emptyDir));
    expect(Exit.isFailure(exit)).toBe(true);
  });
});
