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
 * ## Breaking Change (v2.0.0)
 *
 * Prior to v2.0.0, `bash: true` was used, causing `*` to match across path
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
 * Cache for compiled glob matchers with LRU-style eviction.
 *
 * **Thread Safety Warning**: This cache uses a plain Map which is not thread-safe.
 * Concurrent access from multiple threads (e.g., Web Workers, Bun worker threads)
 * can cause race conditions. If you need thread-safe caching, create isolated
 * cache instances per thread using `new MatcherCache()`.
 *
 * ## Usage
 *
 * ```ts
 * // Default: shared module-level cache (not thread-safe)
 * const matcher = createMatcher("*.ts");
 *
 * // Thread-safe: isolated cache per thread/worker
 * const cache = new MatcherCache();
 * const matcher = createMatcher("*.ts", { cache });
 * ```
 */
export class MatcherCache {
  private cache = new Map<string, (path: string) => boolean>();
  private maxSize: number;

  constructor(maxSize: number = MAX_CACHE_SIZE) {
    this.maxSize = maxSize;
  }

  get(pattern: string): ((path: string) => boolean) | undefined {
    return this.cache.get(pattern);
  }

  set(pattern: string, matcher: (path: string) => boolean): void {
    this.evictIfNeeded();
    this.cache.set(pattern, matcher);
  }

  clear(): void {
    this.cache.clear();
  }

  get size(): number {
    return this.cache.size;
  }

  get limit(): number {
    return this.maxSize;
  }

  private evictIfNeeded(): void {
    if (this.cache.size >= this.maxSize) {
      // Remove oldest 10% of entries
      const toRemove = Math.floor(this.maxSize * 0.1);
      const keys = this.cache.keys();
      for (let i = 0; i < toRemove; i++) {
        const key = keys.next().value;
        if (key !== undefined) {
          this.cache.delete(key);
        }
      }
    }
  }
}

/**
 * Default module-level matcher cache.
 *
 * **Thread Safety Warning**: This is shared mutable state. All callers share
 * this cache, which improves performance in single-threaded environments but
 * is NOT thread-safe. Concurrent access from Web Workers or Bun worker threads
 * can cause race conditions.
 *
 * For thread-safe usage, create isolated cache instances:
 * ```ts
 * const cache = new MatcherCache();
 * const matcher = createMatcher("*.ts", { cache });
 * ```
 */
const defaultMatcherCache = new MatcherCache();

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
 * - Auto-expands basename-only patterns to match anywhere in path
 *
 * This ensures that `src/` and `src` are treated identically as patterns,
 * which matches user expectations for directory patterns.
 *
 * ## Basename Pattern Expansion
 *
 * Patterns without `/` are treated as basename patterns and auto-expanded
 * to match anywhere in the path. This preserves the intuitive behavior
 * where `*.log` matches `src/debug.log`.
 *
 * Examples:
 * - `*.log` → `**\/*.log` (matches debug.log, src/debug.log, a/b/c/debug.log)
 * - `*.d.ts` → `**\/*.d.ts` (matches index.d.ts, types/index.d.ts)
 * - `src/*.ts` → `src/*.ts` (no change - has path separator)
 * - `**\/*.ts` → `**\/*.ts` (no change - already has **)
 */
const normalizePattern = (pattern: string): string => {
  let normalized = pattern.normalize("NFC").replace(/\/$/, "");

  // Auto-expand basename-only patterns (no /) to match anywhere
  // e.g., "*.log" becomes "**/*.log" to match "src/debug.log"
  // But don't expand if it already has path separators or **
  if (!normalized.includes("/")) {
    // Handle negation patterns specially: !*.js -> !**/*.js
    const isNegation = normalized.startsWith("!");
    const patternWithoutNegation = isNegation ? normalized.slice(1) : normalized;

    // Don't expand if already starts with **
    if (!patternWithoutNegation.startsWith("**")) {
      // Only expand if it contains glob characters (*, ?, [...])
      // Note: we check the pattern without ! since ! is the negation operator
      if (/[*?[\]]/.test(patternWithoutNegation)) {
        const expanded = `**/${patternWithoutNegation}`;
        normalized = isNegation ? `!${expanded}` : expanded;
      }
    }
  }

  return normalized;
};

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
 * Options for matcher functions.
 */
export interface MatcherOptions {
  /**
   * Custom cache instance for isolated caching.
   * If not provided, uses the default module-level cache.
   *
   * **Thread Safety**: Use isolated cache instances when working with
   * Web Workers or Bun worker threads to avoid race conditions.
   *
   * @example
   * ```ts
   * const cache = new MatcherCache();
   * const matcher = createMatcher("*.ts", { cache });
   * ```
   */
  cache?: MatcherCache;
}

/**
 * Get or create a cached matcher for a pattern.
 * - Handles empty pattern specially (only matches empty string)
 * - Normalizes pattern before caching
 * - Uses provided cache or default module-level cache
 */
const getCachedMatcher = (
  pattern: string,
  options?: MatcherOptions,
): ((path: string) => boolean) => {
  // Handle empty pattern - use pre-allocated matcher
  if (pattern === "") {
    return EMPTY_PATTERN_MATCHER;
  }

  // Use provided cache or default
  const cache = options?.cache ?? defaultMatcherCache;

  // Normalize pattern (remove trailing slash, normalize unicode)
  const normalizedPattern = normalizePattern(pattern);

  // Check cache first
  let matcher = cache.get(normalizedPattern);
  if (matcher) {
    return matcher;
  }

  // Compile and cache
  matcher = picomatch(normalizedPattern, MATCH_OPTIONS);
  cache.set(normalizedPattern, matcher);
  return matcher;
};

/**
 * Create a matcher function for one or more glob patterns.
 * Matchers are cached for performance when checking many paths.
 *
 * @param patterns - Glob pattern(s) to match against
 * @param options - Optional matcher options (e.g., custom cache for thread safety)
 *
 * @example
 * ```ts
 * // Default: uses shared module-level cache
 * const isTypeScript = createMatcher(["*.ts", "*.tsx"]);
 * isTypeScript("file.ts");  // true
 * isTypeScript("file.js");  // false
 *
 * // Thread-safe: isolated cache
 * const cache = new MatcherCache();
 * const matcher = createMatcher("*.ts", { cache });
 * ```
 */
export const createMatcher = (
  patterns: string | readonly string[],
  options?: MatcherOptions,
): Matcher => {
  const list = Array.isArray(patterns) ? patterns : [patterns];
  if (list.length === 0) return () => false;
  const matchers = list.map((p) => getCachedMatcher(p, options));
  return (path: string) => {
    const normalized = normalizePath(path);
    return matchers.some((m) => m(normalized));
  };
};

/**
 * Check if a path matches a glob pattern.
 * Normalizes the path for cross-platform compatibility (Windows backslashes → forward slashes).
 *
 * @param path - File path to test
 * @param pattern - Glob pattern to match against
 * @param options - Optional matcher options (e.g., custom cache for thread safety)
 *
 * @example
 * ```ts
 * matches("src/file.ts", "src/*.ts");     // true
 * matches("src/sub/file.ts", "src/*.ts"); // false - * doesn't cross /
 * matches("src/sub/file.ts", "src/**");   // true - ** crosses /
 *
 * // Thread-safe usage
 * const cache = new MatcherCache();
 * matches("file.ts", "*.ts", { cache });
 * ```
 */
export const matches = (path: string, pattern: string, options?: MatcherOptions): boolean => {
  const normalized = normalizePath(path);
  return getCachedMatcher(pattern, options)(normalized);
};

/**
 * Effect wrapper for matches().
 */
export const matchesEffect = (
  path: string,
  pattern: string,
  options?: MatcherOptions,
): Effect.Effect<boolean> => Effect.succeed(matches(path, pattern, options));

/**
 * Check if a path matches any of the given patterns.
 *
 * @param path - File path to test
 * @param patterns - Array of glob patterns to match against
 * @param options - Optional matcher options (e.g., custom cache for thread safety)
 *
 * @example
 * ```ts
 * matchesAny("file.ts", ["*.ts", "*.tsx"]); // true
 * matchesAny("file.js", ["*.ts", "*.tsx"]); // false
 *
 * // Thread-safe usage
 * const cache = new MatcherCache();
 * matchesAny("file.ts", ["*.ts", "*.tsx"], { cache });
 * ```
 */
export const matchesAny = (
  path: string,
  patterns: readonly string[],
  options?: MatcherOptions,
): boolean => patterns.length > 0 && createMatcher(patterns, options)(path);

/**
 * Effect wrapper for matchesAny().
 */
export const matchesAnyEffect = (
  path: string,
  patterns: readonly string[],
  options?: MatcherOptions,
): Effect.Effect<boolean> => Effect.succeed(matchesAny(path, patterns, options));

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
 * @param name - File name to test
 * @param pattern - Glob pattern with optional brace expansion
 * @param options - Optional matcher options (e.g., custom cache for thread safety)
 *
 * @throws Error if nested braces are detected in the pattern
 *
 * @example
 * ```ts
 * matchesWithBraces("file.ts", "*.{ts,tsx}");  // true
 * matchesWithBraces("file.tsx", "*.{ts,tsx}"); // true
 * matchesWithBraces("file.js", "*.{ts,tsx}");  // false
 *
 * // Thread-safe usage
 * const cache = new MatcherCache();
 * matchesWithBraces("file.ts", "*.{ts,tsx}", { cache });
 * ```
 */
export const matchesWithBraces = (
  name: string,
  pattern: string,
  options?: MatcherOptions,
): boolean => {
  const expanded = expandBraces(pattern);
  const normalized = normalizePath(name);
  return expanded.some((p) => getCachedMatcher(p, options)(normalized));
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
 * Clear the default matcher cache. Useful for testing or when patterns change.
 *
 * **Note**: This only clears the default module-level cache. If you're using
 * custom cache instances, clear them separately using `cache.clear()`.
 */
export const clearMatcherCache = (): void => {
  defaultMatcherCache.clear();
};

/**
 * Get current size of the default matcher cache. Useful for monitoring/debugging.
 *
 * **Note**: This only returns the size of the default module-level cache.
 * For custom cache instances, use `cache.size`.
 */
export const getMatcherCacheSize = (): number => defaultMatcherCache.size;

/**
 * Get the maximum cache size limit for the default cache.
 *
 * **Note**: This returns the limit for the default module-level cache.
 * For custom cache instances, use `cache.limit`.
 */
export const getMaxCacheSize = (): number => defaultMatcherCache.limit;
