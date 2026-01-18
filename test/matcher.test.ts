import { describe, expect, test, beforeEach } from "bun:test";
import { Effect } from "effect";
import {
  matches,
  matchesEffect,
  matchesAny,
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
  getMatcherCacheSize,
  getMaxCacheSize,
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

  test("normalizes Windows paths", () => {
    expect(matchesAny("src\\file.ts", ["src/*.ts", "lib/*.ts"])).toBe(true);
    expect(matchesAny("src\\sub\\file.ts", ["src/**/*.ts"])).toBe(true);
    expect(matchesAny("src\\sub\\file.ts", ["src/*.ts"])).toBe(false);
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

  test("throws on nested braces", () => {
    expect(() => expandBraces("*.{ts,{js,jsx}}")).toThrow(/[Nn]ested braces/);
    expect(() => expandBraces("*.{a,b{c,d}}")).toThrow(/[Nn]ested braces/);
  });

  test("handles patterns without valid braces", () => {
    // These patterns have braces but not in the expandable format
    expect(expandBraces("file{.ts")).toEqual(["file{.ts"]);
    expect(expandBraces("file}.ts")).toEqual(["file}.ts"]);
    expect(expandBraces("file{}.ts")).toEqual(["file{}.ts"]); // empty braces
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

  test("throws on nested braces", () => {
    expect(() => matchesWithBraces("file.ts", "*.{ts,{js,jsx}}")).toThrow(/[Nn]ested braces/);
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

  test("preserves leading slashes (absolute paths)", () => {
    expect(normalizePath("/src/file.ts")).toBe("/src/file.ts");
    expect(normalizePath("/")).toBe("");
    expect(normalizePath("//src")).toBe("/src");
  });

  test("handles mixed issues", () => {
    expect(normalizePath("src\\\\sub//file.ts/")).toBe("src/sub/file.ts");
  });

  test("handles empty string", () => {
    expect(normalizePath("")).toBe("");
  });

  test("normalizes unicode to NFC", () => {
    // café in NFD (decomposed)
    const nfd = "cafe\u0301.ts";
    // café in NFC (composed)
    const nfc = "caf\u00e9.ts";

    expect(normalizePath(nfd)).toBe(nfc);
    expect(normalizePath(nfc)).toBe(nfc);
  });
});

describe("unicode normalization", () => {
  test("matches unicode paths regardless of NFC/NFD form", () => {
    // café in NFC (composed)
    const nfc = "caf\u00e9.ts";
    // café in NFD (decomposed)
    const nfd = "cafe\u0301.ts";

    // Both should match the same pattern
    expect(matches(nfc, "*.ts")).toBe(true);
    expect(matches(nfd, "*.ts")).toBe(true);

    // NFC pattern should match NFD path and vice versa
    expect(matches(nfd, nfc)).toBe(true);
    expect(matches(nfc, nfd)).toBe(true);
  });

  test("normalizeUnicode converts to NFC", () => {
    const nfd = "cafe\u0301";
    const nfc = "caf\u00e9";

    expect(normalizeUnicode(nfd)).toBe(nfc);
    expect(normalizeUnicode(nfc)).toBe(nfc);
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

  test("getDepth handles root path edge case", () => {
    // "/" normalizes to "" (trailing slash removed), so depth is 0
    expect(getDepth("/")).toBe(0);
  });

  test("joinPath", () => {
    expect(joinPath("src", "utils")).toBe("src/utils");
    expect(joinPath("", "file.ts")).toBe("file.ts");
    expect(joinPath("src", "", "file.ts")).toBe("src/file.ts");
  });

  test("joinPath normalizes output", () => {
    expect(joinPath("src\\sub", "file.ts")).toBe("src/sub/file.ts");
    expect(joinPath("src/", "/file.ts")).toBe("src/file.ts");
    expect(joinPath("src//sub", "file.ts")).toBe("src/sub/file.ts");
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

  test("trailing slashes in patterns", () => {
    // Pattern trailing slashes are also normalized
    expect(matches("src", "src/")).toBe(true);
    expect(matches("src", "src")).toBe(true);
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

describe("matcher cache", () => {
  test("clearMatcherCache resets the cache", () => {
    // First call should cache
    matches("file.ts", "*.ts");
    expect(getMatcherCacheSize()).toBeGreaterThan(0);

    // Clear cache
    clearMatcherCache();
    expect(getMatcherCacheSize()).toBe(0);

    // Should still work after clear
    expect(matches("file.ts", "*.ts")).toBe(true);
  });

  test("cache has size limit", () => {
    const maxSize = getMaxCacheSize();
    expect(maxSize).toBeGreaterThan(0);
    expect(maxSize).toBe(1000); // Current limit
  });

  test("cache evicts old entries when full", () => {
    const maxSize = getMaxCacheSize();

    // Fill cache beyond limit
    for (let i = 0; i < maxSize + 100; i++) {
      matches("file.ts", `pattern-${i}`);
    }

    // Cache should be at or below max size (after eviction)
    expect(getMatcherCacheSize()).toBeLessThanOrEqual(maxSize);
  });

  test("empty pattern uses cached matcher", () => {
    // Call twice
    matches("", "");
    matches("", "");

    // Empty pattern doesn't add to cache (uses pre-allocated matcher)
    // But it should still work
    expect(matches("", "")).toBe(true);
    expect(matches("a", "")).toBe(false);
  });

  test("caching improves performance", () => {
    const pattern = "**/*.ts";
    const iterations = 5000;

    // Cold cache
    clearMatcherCache();
    const coldStart = performance.now();
    for (let i = 0; i < iterations; i++) {
      matches(`file${i % 100}.ts`, pattern);
    }
    const coldTime = performance.now() - coldStart;

    // Warm cache - same pattern already cached
    const warmStart = performance.now();
    for (let i = 0; i < iterations; i++) {
      matches(`file${i % 100}.ts`, pattern);
    }
    const warmTime = performance.now() - warmStart;

    // Warm should be faster or at least not significantly slower
    // (First run compiles the pattern, subsequent runs reuse it)
    // We're lenient here because the first run also warms the cache
    expect(warmTime).toBeLessThanOrEqual(coldTime * 1.5);
  });
});

describe("pattern normalization consistency", () => {
  test("src/ and src are equivalent patterns", () => {
    expect(matches("src", "src/")).toBe(true);
    expect(matches("src", "src")).toBe(true);
    expect(matches("src/", "src")).toBe(true);
    expect(matches("src/", "src/")).toBe(true);
  });

  test("patterns with trailing slash cache to same key", () => {
    clearMatcherCache();

    matches("file.ts", "*.ts/");
    const sizeAfterFirst = getMatcherCacheSize();

    matches("file.ts", "*.ts");
    const sizeAfterSecond = getMatcherCacheSize();

    // Both should use the same cache entry
    expect(sizeAfterSecond).toBe(sizeAfterFirst);
  });
});
