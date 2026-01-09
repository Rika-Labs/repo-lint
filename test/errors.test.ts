import { describe, expect, test } from "bun:test";
import { Effect, Exit, Cause } from "effect";
import {
  ConfigNotFoundError,
  ConfigParseError,
  ConfigValidationError,
  FileSystemError,
  ScanError,
} from "../src/errors.js";

describe("ConfigNotFoundError", () => {
  test("creates tagged error with path", () => {
    const error = new ConfigNotFoundError({ path: "/some/path" });
    expect(error._tag).toBe("ConfigNotFoundError");
    expect(error.path).toBe("/some/path");
  });

  test("works with Effect.fail", async () => {
    const program = Effect.fail(new ConfigNotFoundError({ path: "/test" }));
    const exit = await Effect.runPromiseExit(program);

    expect(Exit.isFailure(exit)).toBe(true);
    if (Exit.isFailure(exit)) {
      const maybeError = Cause.failureOption(exit.cause);
      expect(maybeError._tag).toBe("Some");
    }
  });
});

describe("ConfigParseError", () => {
  test("creates tagged error with path and cause", () => {
    const cause = new Error("parse failed");
    const error = new ConfigParseError({ path: "/config.yaml", cause });
    expect(error._tag).toBe("ConfigParseError");
    expect(error.path).toBe("/config.yaml");
    expect(error.cause).toBe(cause);
  });
});

describe("ConfigValidationError", () => {
  test("creates tagged error with validation errors", () => {
    const error = new ConfigValidationError({
      path: "/config.yaml",
      errors: ["invalid mode", "missing layout"],
    });
    expect(error._tag).toBe("ConfigValidationError");
    expect(error.errors).toHaveLength(2);
  });
});

describe("FileSystemError", () => {
  test("creates tagged error with operation details", () => {
    const error = new FileSystemError({
      path: "/file.ts",
      operation: "read",
      cause: new Error("ENOENT"),
    });
    expect(error._tag).toBe("FileSystemError");
    expect(error.operation).toBe("read");
  });

  test("can be caught with Effect.catchTag", async () => {
    const program = Effect.fail(
      new FileSystemError({ path: "/test", operation: "read", cause: "failed" })
    ).pipe(Effect.catchTag("FileSystemError", (e) => Effect.succeed(`caught: ${e.path}`)));

    const result = await Effect.runPromise(program);
    expect(result).toBe("caught: /test");
  });
});

describe("ScanError", () => {
  test("creates tagged error with root", () => {
    const error = new ScanError({ root: "/project", cause: new Error("failed") });
    expect(error._tag).toBe("ScanError");
    expect(error.root).toBe("/project");
  });
});
