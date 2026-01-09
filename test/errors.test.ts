import { describe, expect, test } from "bun:test";
import { Effect, Exit, Cause } from "effect";
import {
  ConfigNotFoundError,
  ConfigParseError,
  ConfigValidationError,
  CircularExtendsError,
  PathTraversalError,
  FileSystemError,
  ScanError,
  SymlinkLoopError,
  MaxDepthExceededError,
  MaxFilesExceededError,
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

  test("has descriptive message", () => {
    const error = new ConfigNotFoundError({ path: "/test/path" });
    expect(error.message).toContain("/test/path");
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

describe("CircularExtendsError", () => {
  test("creates tagged error with chain", () => {
    const error = new CircularExtendsError({
      path: "/c.yaml",
      chain: ["/a.yaml", "/b.yaml"],
    });
    expect(error._tag).toBe("CircularExtendsError");
    expect(error.chain).toHaveLength(2);
    expect(error.message).toContain("/a.yaml");
    expect(error.message).toContain("/b.yaml");
    expect(error.message).toContain("/c.yaml");
  });
});

describe("PathTraversalError", () => {
  test("creates tagged error with path info", () => {
    const error = new PathTraversalError({
      path: "../../../etc/passwd",
      configPath: "/project/config.yaml",
    });
    expect(error._tag).toBe("PathTraversalError");
    expect(error.message).toContain("traversal");
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
      new FileSystemError({ path: "/test", operation: "read", cause: "failed" }),
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

describe("SymlinkLoopError", () => {
  test("creates tagged error with symlink info", () => {
    const error = new SymlinkLoopError({ path: "/link", target: "/parent" });
    expect(error._tag).toBe("SymlinkLoopError");
    expect(error.message).toContain("loop");
  });
});

describe("MaxDepthExceededError", () => {
  test("creates tagged error with depth info", () => {
    const error = new MaxDepthExceededError({ path: "/deep/path", depth: 150, maxDepth: 100 });
    expect(error._tag).toBe("MaxDepthExceededError");
    expect(error.message).toContain("150");
    expect(error.message).toContain("100");
  });
});

describe("MaxFilesExceededError", () => {
  test("creates tagged error with count info", () => {
    const error = new MaxFilesExceededError({ count: 200000, maxFiles: 100000 });
    expect(error._tag).toBe("MaxFilesExceededError");
    expect(error.message).toContain("200000");
  });
});
