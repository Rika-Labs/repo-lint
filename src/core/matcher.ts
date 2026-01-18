import { Effect, Option, Array as A } from "effect";
import picomatch from "picomatch";

export type Matcher = (path: string) => boolean;

/**
 * Picomatch options for consistent path matching across all matcher functions.
 *
 * ## Why these options?
 *
 * - `dot: true` - Match dotfiles (.gitignore, .env, etc.). Required for repo
 *   structure validation since config files are often hidden.
 *
 * - `bash: false` - Ensure `*` only matches within a single path segment.
 *   With `bash: true`, `*` behaves like `**` and matches across `/`, which
 *   breaks patterns like `modules/*` (would incorrectly match `modules/a/b`).
 *
 * ## Breaking Change (v1.1.1)
 *
 * Prior to v1.1.1, `bash: true` was used, causing `*` to match across path
 * separators. If you relied on this behavior, update your patterns to use `**`.
 *
 * @see https://github.com/Rika-Labs/repo-lint/pull/3
 */
const MATCH_OPTIONS = {
  dot: true,
  bash: false,
} as const;

/**
 * Maximum number of patterns to cache.
 * Prevents memory leaks from unbounded cache growth.
 * Uses simple LRU-style eviction (removes oldest entries when limit reached).
 */
const MAX_CACHE_SIZE = 1000;

/**
 * LRU-style cache for compiled matchers.
 * - Keys are normalized pattern strings
 * - Values are compiled matcher functions
 * - Automatically evicts oldest entries when MAX_CACHE_SIZE is reached
 *
 * Note: This is module-level state. In a library context, all callers share
 * this cache. This is intentional for performance but means patterns are
 * cached globally.
 */
const matcherCache = new Map<string, (path: string) => boolean>();

/**
 * Cached empty pattern matcher (avoids creating new function each call).
 */
const EMPTY_PATTERN_MATCHER = (path: string): boolean => path === "";

/**
 * Normalize a path for consistent matching across platforms.
 * - Converts Windows backslashes to forward slashes
 * - Removes duplicate slashes
 * - Removes trailing slashes
 * - Normalizes unicode to NFC form
 *
 * Note: Leading slashes are PRESERVED. An absolute path `/src/file.ts`
 * stays absolute and will NOT match a relative pattern like `src/*.ts`.
 * This is intentional - absolute and relative paths are semantically different.
 */
export const normalizePath = (p: string): string =>
  p
    .normalize("NFC") // Unicode normalization
    .replace(/\\/g, "/")
    .replace(/\/+/g, "/")
    .replace(/\/$/, "");

/**
 * Normalize a glob pattern for consistent matching.
 * - Removes trailing slashes (directories don't need them in patterns)
 * - Normalizes unicode to NFC form
 *
 * This ensures that `src/` and `src` are treated identically as patterns,
 * which matches user expectations for directory patterns.
 */
const normalizePattern = (pattern: string): string =>
  pattern.normalize("NFC").replace(/\/$/, "");

/**
 * Validate a brace pattern for correctness.
 * Throws if nested braces are detected (not supported).
 */
const validateBracePattern = (pattern: string): void => {
  const braceMatch = pattern.match(/\{([^}]*)\}/);
  if (braceMatch?.[1]?.includes("{")) {
    throw new Error(
      `Nested braces are not supported in pattern: "${pattern}". ` +
        `Use multiple patterns instead, e.g., ["*.ts", "*.js", "*.jsx"] instead of "*.{ts,{js,jsx}}"`
    );
  }
};

/**
 * Evict oldest cache entries if cache exceeds max size.
 * Simple LRU-style eviction - removes first (oldest) entries.
 */
const evictIfNeeded = (): void => {
  if (matcherCache.size >= MAX_CACHE_SIZE) {
    // Remove oldest 10% of entries
    const toRemove = Math.floor(MAX_CACHE_SIZE * 0.1);
    const keys = matcherCache.keys();
    for (let i = 0; i < toRemove; i++) {
      const key = keys.next().value;
      if (key !== undefined) {
        matcherCache.delete(key);
      }
    }
  }
};

/**
 * Get or create a cached matcher for a pattern.
 * - Handles empty pattern specially (only matches empty string)
 * - Normalizes pattern before caching
 * - Evicts old entries if cache is full
 */
const getCachedMatcher = (pattern: string): ((path: string) => boolean) => {
  // Handle empty pattern - use pre-allocated matcher
  if (pattern === "") {
    return EMPTY_PATTERN_MATCHER;
  }

  // Normalize pattern (remove trailing slash, normalize unicode)
  const normalizedPattern = normalizePattern(pattern);

  // Check cache first
  let matcher = matcherCache.get(normalizedPattern);
  if (matcher) {
    return matcher;
  }

  // Evict old entries if needed before adding new one
  evictIfNeeded();

  // Compile and cache
  matcher = picomatch(normalizedPattern, MATCH_OPTIONS);
  matcherCache.set(normalizedPattern, matcher);
  return matcher;
};

/**
 * Create a matcher function for one or more glob patterns.
 * Matchers are cached for performance when checking many paths.
 *
 * @example
 * ```ts
 * const isTypeScript = createMatcher(["*.ts", "*.tsx"]);
 * isTypeScript("file.ts");  // true
 * isTypeScript("file.js");  // false
 * ```
 */
export const createMatcher = (patterns: string | readonly string[]): Matcher => {
  const list = Array.isArray(patterns) ? patterns : [patterns];
  if (list.length === 0) return () => false;
  const matchers = list.map((p) => getCachedMatcher(p));
  return (path: string) => {
    const normalized = normalizePath(path);
    return matchers.some((m) => m(normalized));
  };
};

/**
 * Check if a path matches a glob pattern.
 * Normalizes the path for cross-platform compatibility (Windows backslashes → forward slashes).
 *
 * @example
 * ```ts
 * matches("src/file.ts", "src/*.ts");     // true
 * matches("src/sub/file.ts", "src/*.ts"); // false - * doesn't cross /
 * matches("src/sub/file.ts", "src/**");   // true - ** crosses /
 * ```
 */
export const matches = (path: string, pattern: string): boolean => {
  const normalized = normalizePath(path);
  return getCachedMatcher(pattern)(normalized);
};

/**
 * Effect wrapper for matches().
 */
export const matchesEffect = (path: string, pattern: string): Effect.Effect<boolean> =>
  Effect.succeed(matches(path, pattern));

/**
 * Check if a path matches any of the given patterns.
 *
 * @example
 * ```ts
 * matchesAny("file.ts", ["*.ts", "*.tsx"]); // true
 * matchesAny("file.js", ["*.ts", "*.tsx"]); // false
 * ```
 */
export const matchesAny = (path: string, patterns: readonly string[]): boolean =>
  patterns.length > 0 && createMatcher(patterns)(path);

/**
 * Effect wrapper for matchesAny().
 */
export const matchesAnyEffect = (
  path: string,
  patterns: readonly string[],
): Effect.Effect<boolean> => Effect.succeed(matchesAny(path, patterns));

/**
 * Expand simple brace patterns like `*.{ts,tsx}` into multiple patterns.
 * Only handles single-level brace expansion (not nested).
 *
 * @throws Error if nested braces are detected
 *
 * @example
 * ```ts
 * expandBraces("*.{ts,tsx}"); // ["*.ts", "*.tsx"]
 * expandBraces("*.ts");       // ["*.ts"]
 * expandBraces("*.{a,{b,c}}"); // throws Error - nested braces not supported
 * ```
 */
export const expandBraces = (pattern: string): readonly string[] => {
  validateBracePattern(pattern);

  const match = pattern.match(/\{([^}]+)\}/);
  if (!match?.[1]) return [pattern];
  const options = match[1].split(",");
  return options.map((opt) => pattern.replace(match[0], opt));
};

/**
 * Match a name against a pattern with brace expansion support.
 * Useful for patterns like `*.{ts,tsx}` in layout definitions.
 *
 * @throws Error if nested braces are detected in the pattern
 *
 * @example
 * ```ts
 * matchesWithBraces("file.ts", "*.{ts,tsx}");  // true
 * matchesWithBraces("file.tsx", "*.{ts,tsx}"); // true
 * matchesWithBraces("file.js", "*.{ts,tsx}");  // false
 * ```
 */
export const matchesWithBraces = (name: string, pattern: string): boolean => {
  const expanded = expandBraces(pattern);
  const normalized = normalizePath(name);
  return expanded.some((p) => getCachedMatcher(p)(normalized));
};

/**
 * Get the basename (last segment) of a path.
 *
 * @example
 * ```ts
 * getBasename("src/utils/helper.ts"); // "helper.ts"
 * getBasename("file.ts");             // "file.ts"
 * ```
 */
export const getBasename = (p: string): string => {
  const normalized = normalizePath(p);
  const parts = normalized.split("/");
  return A.last(parts).pipe(Option.getOrElse(() => ""));
};

/**
 * Get the parent directory of a path.
 *
 * @example
 * ```ts
 * getParent("src/utils/helper.ts"); // "src/utils"
 * getParent("file.ts");             // ""
 * ```
 */
export const getParent = (p: string): string => {
  const normalized = normalizePath(p);
  const parts = normalized.split("/");
  return parts.slice(0, -1).join("/");
};

/**
 * Get the depth (number of segments) of a path.
 *
 * @example
 * ```ts
 * getDepth("");          // 0
 * getDepth("src");       // 1
 * getDepth("src/utils"); // 2
 * ```
 */
export const getDepth = (p: string): number => {
  if (p === "") return 0;
  const normalized = normalizePath(p);
  // Second check needed because normalizePath("/") returns ""
  // (trailing slash removed from root path)
  return normalized === "" ? 0 : normalized.split("/").length;
};

/**
 * Join path segments, filtering out empty strings.
 * Normalizes the result for consistent output.
 *
 * @example
 * ```ts
 * joinPath("src", "utils");       // "src/utils"
 * joinPath("", "file.ts");        // "file.ts"
 * joinPath("src\\sub", "file");   // "src/sub/file" (normalized)
 * ```
 */
export const joinPath = (...parts: readonly string[]): string => {
  const joined = parts.filter(Boolean).join("/");
  return normalizePath(joined);
};

/**
 * Normalize unicode strings to NFC form for consistent comparison.
 * Ensures that characters like é (composed) and e + ́ (decomposed) are treated identically.
 *
 * Note: This is automatically applied by normalizePath() and normalizePattern(),
 * so you typically don't need to call this directly.
 */
export const normalizeUnicode = (s: string): string => s.normalize("NFC");

/**
 * Clear the matcher cache. Useful for testing or when patterns change.
 */
export const clearMatcherCache = (): void => {
  matcherCache.clear();
};

/**
 * Get current cache size. Useful for monitoring/debugging.
 */
export const getMatcherCacheSize = (): number => matcherCache.size;

/**
 * Get the maximum cache size limit.
 */
export const getMaxCacheSize = (): number => MAX_CACHE_SIZE;
