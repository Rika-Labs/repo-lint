import { describe, expect, test, beforeEach } from "bun:test";
import { Effect } from "effect";
import {
  matches,
  matchesEffect,
  matchesAnyEffect,
  createMatcher,
  expandBraces,
  matchesWithBraces,
  getBasename,
  getParent,
  getDepth,
  joinPath,
  normalizePath,
  normalizeUnicode,
  clearMatcherCache,
} from "../src/core/matcher.js";

// Clear cache before each test for isolation
beforeEach(() => {
  clearMatcherCache();
});

describe("matches", () => {
  test("simple glob", async () => {
    const result = await Effect.runPromise(
      Effect.all([
        matchesEffect("src/index.ts", "src/*.ts"),
        matchesEffect("src/utils/helper.ts", "src/**/*.ts"),
        matchesEffect("other/file.ts", "src/**/*.ts"),
      ]),
    );
    expect(result).toEqual([true, true, false]);
  });

  test("double star", async () => {
    const result = await Effect.runPromise(
      Effect.all([
        matchesEffect("a/b/c/d.ts", "**/*.ts"),
        matchesEffect("d.ts", "**/*.ts"),
      ]),
    );
    expect(result).toEqual([true, true]);
  });

  // CRITICAL: Regression test for the bash:true bug
  // See: https://github.com/Rika-Labs/repo-lint/pull/3
  test("* does NOT match across path separators", () => {
    // This was the bug: with bash:true, * matched like **
    expect(matches("modules/chat", "modules/*")).toBe(true);
    expect(matches("modules/chat/stream", "modules/*")).toBe(false);
    expect(matches("src/file.ts", "src/*.ts")).toBe(true);
    expect(matches("src/sub/file.ts", "src/*.ts")).toBe(false);
    
    // ** SHOULD match across separators
    expect(matches("modules/chat/stream", "modules/**")).toBe(true);
    expect(matches("src/sub/file.ts", "src/**/*.ts")).toBe(true);
  });

  test("matches dotfiles with dot:true", () => {
    expect(matches(".gitignore", "*")).toBe(true);
    expect(matches(".env", ".*")).toBe(true);
    expect(matches(".eslintrc.json", ".*")).toBe(true);
    expect(matches("src/.hidden", "src/.*")).toBe(true);
  });
});

describe("Windows path compatibility", () => {
  test("normalizes Windows backslashes to forward slashes", () => {
    // Windows paths should work after normalization
    expect(matches("src\\file.ts", "src/*.ts")).toBe(true);
    expect(matches("src\\sub\\file.ts", "src/**/*.ts")).toBe(true);
    expect(matches("src\\sub\\file.ts", "src/*.ts")).toBe(false);
  });

  test("handles double backslashes", () => {
    expect(matches("src\\\\file.ts", "src/*.ts")).toBe(true);
  });

  test("handles mixed slashes", () => {
    expect(matches("src\\sub/file.ts", "src/**/*.ts")).toBe(true);
    expect(matches("src/sub\\file.ts", "src/**/*.ts")).toBe(true);
  });
});

describe("matchesAny", () => {
  test("matches any pattern", async () => {
    const result = await Effect.runPromise(
      Effect.all([
        matchesAnyEffect("src/index.ts", ["**/*.ts", "**/*.js"]),
        matchesAnyEffect("src/index.ts", ["src/*.ts"]),
        matchesAnyEffect("other/file.txt", ["**/*.ts", "**/*.js"]),
      ]),
    );
    expect(result).toEqual([true, true, false]);
  });

  test("empty patterns returns false", async () => {
    const result = await Effect.runPromise(matchesAnyEffect("anything", []));
    expect(result).toBe(false);
  });
});

describe("createMatcher", () => {
  test("creates reusable matcher", () => {
    const isTypeScript = createMatcher(["*.ts", "*.tsx"]);
    expect(isTypeScript("file.ts")).toBe(true);
    expect(isTypeScript("file.tsx")).toBe(true);
    expect(isTypeScript("file.js")).toBe(false);
  });

  test("handles single pattern as string", () => {
    const isTsFile = createMatcher("*.ts");
    expect(isTsFile("file.ts")).toBe(true);
    expect(isTsFile("file.js")).toBe(false);
  });

  test("empty patterns returns always-false matcher", () => {
    const neverMatch = createMatcher([]);
    expect(neverMatch("anything")).toBe(false);
    expect(neverMatch("")).toBe(false);
  });

  test("normalizes Windows paths", () => {
    const matcher = createMatcher("src/*.ts");
    expect(matcher("src\\file.ts")).toBe(true);
    expect(matcher("src\\sub\\file.ts")).toBe(false);
  });
});

describe("expandBraces", () => {
  test("expands brace patterns", () => {
    expect(expandBraces("*.{ts,tsx}")).toEqual(["*.ts", "*.tsx"]);
    expect(expandBraces("*.ts")).toEqual(["*.ts"]);
  });

  test("handles multiple options", () => {
    expect(expandBraces("*.{ts,tsx,js,jsx}")).toEqual(["*.ts", "*.tsx", "*.js", "*.jsx"]);
  });

  test("handles no braces", () => {
    expect(expandBraces("plain-pattern")).toEqual(["plain-pattern"]);
  });
});

describe("matchesWithBraces", () => {
  test("matches with brace expansion", () => {
    expect(matchesWithBraces("file.ts", "*.{ts,tsx}")).toBe(true);
    expect(matchesWithBraces("file.tsx", "*.{ts,tsx}")).toBe(true);
    expect(matchesWithBraces("file.js", "*.{ts,tsx}")).toBe(false);
  });

  test("normalizes Windows paths", () => {
    expect(matchesWithBraces("src\\file.ts", "src/*.{ts,tsx}")).toBe(true);
  });

  // This was silently different before - matchesWithBraces didn't have bash option
  test("* does not match across separators (consistent with matches)", () => {
    expect(matchesWithBraces("src/sub/file.ts", "src/*.ts")).toBe(false);
    expect(matchesWithBraces("src/file.ts", "src/*.ts")).toBe(true);
  });
});

describe("normalizePath", () => {
  test("converts backslashes to forward slashes", () => {
    expect(normalizePath("src\\file.ts")).toBe("src/file.ts");
    expect(normalizePath("src\\sub\\file.ts")).toBe("src/sub/file.ts");
  });

  test("removes duplicate slashes", () => {
    expect(normalizePath("src//file.ts")).toBe("src/file.ts");
    expect(normalizePath("src///sub//file.ts")).toBe("src/sub/file.ts");
  });

  test("removes trailing slashes", () => {
    expect(normalizePath("src/")).toBe("src");
    expect(normalizePath("src/sub/")).toBe("src/sub");
  });

  test("handles mixed issues", () => {
    expect(normalizePath("src\\\\sub//file.ts/")).toBe("src/sub/file.ts");
  });

  test("handles empty string", () => {
    expect(normalizePath("")).toBe("");
  });
});

describe("path utilities", () => {
  test("getBasename", () => {
    expect(getBasename("src/utils/helper.ts")).toBe("helper.ts");
    expect(getBasename("file.ts")).toBe("file.ts");
    expect(getBasename("")).toBe("");
  });

  test("getBasename normalizes Windows paths", () => {
    expect(getBasename("src\\utils\\helper.ts")).toBe("helper.ts");
  });

  test("getParent", () => {
    expect(getParent("src/utils/helper.ts")).toBe("src/utils");
    expect(getParent("file.ts")).toBe("");
    expect(getParent("")).toBe("");
  });

  test("getParent normalizes Windows paths", () => {
    expect(getParent("src\\utils\\helper.ts")).toBe("src/utils");
  });

  test("getDepth", () => {
    expect(getDepth("")).toBe(0);
    expect(getDepth("src")).toBe(1);
    expect(getDepth("src/utils")).toBe(2);
    expect(getDepth("src/utils/deep")).toBe(3);
  });

  test("getDepth normalizes Windows paths", () => {
    expect(getDepth("src\\utils")).toBe(2);
    expect(getDepth("src\\utils\\deep")).toBe(3);
  });

  test("joinPath", () => {
    expect(joinPath("src", "utils")).toBe("src/utils");
    expect(joinPath("", "file.ts")).toBe("file.ts");
    expect(joinPath("src", "", "file.ts")).toBe("src/file.ts");
  });
});

describe("edge cases", () => {
  test("empty string paths", () => {
    expect(matches("", "*")).toBe(false);
    expect(matches("", "**")).toBe(false);
    expect(matches("", "")).toBe(true); // empty matches empty
  });

  test("paths with just dots", () => {
    expect(matches(".", "*")).toBe(false); // single dot is special
    expect(matches("..", "*")).toBe(false); // double dot is special
    expect(matches("...", "*")).toBe(true); // three dots is a valid name
    expect(matches(".", ".*")).toBe(false); // dot alone doesn't match .*
  });

  test("trailing slashes in paths", () => {
    // Trailing slashes are normalized away
    expect(matches("src/", "src")).toBe(true);
    expect(matches("src/", "src/")).toBe(true);
  });

  test("leading slashes (absolute paths)", () => {
    // Leading slash is preserved - pattern must include it
    expect(matches("/src/file.ts", "src/*.ts")).toBe(false);
    expect(matches("/src/file.ts", "/src/*.ts")).toBe(true);
  });

  test("special characters in paths", () => {
    expect(matches("file[1].ts", "file[1].ts")).toBe(true);
    expect(matches("file(1).ts", "file(1).ts")).toBe(true);
    expect(matches("file$1.ts", "file$1.ts")).toBe(true);
    expect(matches("file 1.ts", "file 1.ts")).toBe(true); // spaces
  });

  test("negation patterns", () => {
    expect(matches("file.ts", "!*.js")).toBe(true);
    expect(matches("file.js", "!*.js")).toBe(false);
  });

  test("extglob patterns", () => {
    expect(matches("file.ts", "*.+(ts|tsx)")).toBe(true);
    expect(matches("file.tsx", "*.+(ts|tsx)")).toBe(true);
    expect(matches("file.js", "*.+(ts|tsx)")).toBe(false);
  });

  test("question mark (single character)", () => {
    expect(matches("ab", "a?")).toBe(true);
    expect(matches("abc", "a?")).toBe(false);
    expect(matches("a", "a?")).toBe(false);
  });

  test("character classes", () => {
    expect(matches("a1", "[a-z][0-9]")).toBe(true);
    expect(matches("A1", "[a-z][0-9]")).toBe(false); // case sensitive
    expect(matches("z9", "[a-z][0-9]")).toBe(true);
  });
});

describe("normalizeUnicode", () => {
  test("normalizes unicode strings", () => {
    // café in composed form (NFC)
    const composed = "caf\u00e9";
    // café in decomposed form (NFD)
    const decomposed = "cafe\u0301";

    expect(normalizeUnicode(composed)).toBe(normalizeUnicode(decomposed));
    expect(composed).not.toBe(decomposed); // They're different without normalization
  });
});

describe("matcher cache", () => {
  test("clearMatcherCache resets the cache", () => {
    // First call should cache
    matches("file.ts", "*.ts");
    
    // Clear cache
    clearMatcherCache();
    
    // Should still work after clear
    expect(matches("file.ts", "*.ts")).toBe(true);
  });
});
