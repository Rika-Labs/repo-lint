import { describe, expect, test } from "bun:test";
import { Effect } from "effect";
import {
  matchesEffect,
  matchesAnyEffect,
  expandBraces,
  matchesWithBraces,
  getBasename,
  getParent,
  getDepth,
  joinPath,
} from "../src/matcher.js";

describe("matches", () => {
  test("simple glob", async () => {
    const result = await Effect.runPromise(
      Effect.all([
        matchesEffect("src/index.ts", "src/*.ts"),
        matchesEffect("src/utils/helper.ts", "src/**/*.ts"),
        matchesEffect("other/file.ts", "src/**/*.ts"),
      ])
    );
    expect(result).toEqual([true, true, false]);
  });

  test("double star", async () => {
    const result = await Effect.runPromise(
      Effect.all([
        matchesEffect("a/b/c/d.ts", "**/*.ts"),
        matchesEffect("d.ts", "**/*.ts"),
      ])
    );
    expect(result).toEqual([true, true]);
  });
});

describe("matchesAny", () => {
  test("matches any pattern", async () => {
    const result = await Effect.runPromise(
      Effect.all([
        matchesAnyEffect("src/index.ts", ["**/*.ts", "**/*.js"]),
        matchesAnyEffect("src/index.ts", ["src/*.ts"]),
        matchesAnyEffect("other/file.txt", ["**/*.ts", "**/*.js"]),
      ])
    );
    expect(result).toEqual([true, true, false]);
  });

  test("empty patterns returns false", async () => {
    const result = await Effect.runPromise(matchesAnyEffect("anything", []));
    expect(result).toBe(false);
  });
});

describe("expandBraces", () => {
  test("expands brace patterns", () => {
    expect(expandBraces("*.{ts,tsx}")).toEqual(["*.ts", "*.tsx"]);
    expect(expandBraces("*.ts")).toEqual(["*.ts"]);
  });
});

describe("matchesWithBraces", () => {
  test("matches with brace expansion", () => {
    expect(matchesWithBraces("file.ts", "*.{ts,tsx}")).toBe(true);
    expect(matchesWithBraces("file.tsx", "*.{ts,tsx}")).toBe(true);
    expect(matchesWithBraces("file.js", "*.{ts,tsx}")).toBe(false);
  });
});

describe("path utilities", () => {
  test("getBasename", () => {
    expect(getBasename("src/utils/helper.ts")).toBe("helper.ts");
    expect(getBasename("file.ts")).toBe("file.ts");
  });

  test("getParent", () => {
    expect(getParent("src/utils/helper.ts")).toBe("src/utils");
    expect(getParent("file.ts")).toBe("");
  });

  test("getDepth", () => {
    expect(getDepth("")).toBe(0);
    expect(getDepth("src")).toBe(1);
    expect(getDepth("src/utils")).toBe(2);
  });

  test("joinPath", () => {
    expect(joinPath("src", "utils")).toBe("src/utils");
    expect(joinPath("", "file.ts")).toBe("file.ts");
  });
});
