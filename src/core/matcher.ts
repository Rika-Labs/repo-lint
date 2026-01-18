import { Effect, Option, Array as A } from "effect";
import picomatch from "picomatch";

export type Matcher = (path: string) => boolean;

/**
 * Picomatch options type (defined inline since @types/picomatch may not be available)
 */
interface PicomatchOptions {
  dot?: boolean;
  bash?: boolean;
  nobrace?: boolean;
  noglobstar?: boolean;
  noextglob?: boolean;
  nocase?: boolean;
  matchBase?: boolean;
}

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
const MATCH_OPTIONS: PicomatchOptions = {
  dot: true,
  bash: false,
};

/**
 * Cache for compiled matchers to avoid recompiling the same pattern.
 * Key is the pattern string, value is the compiled matcher function.
 */
const matcherCache = new Map<string, (path: string) => boolean>();

/**
 * Normalize a path for consistent matching across platforms.
 * - Converts Windows backslashes to forward slashes
 * - Removes duplicate slashes
 * - Removes trailing slashes
 */
export const normalizePath = (p: string): string =>
  p.replace(/\\/g, "/").replace(/\/+/g, "/").replace(/\/$/, "");

/**
 * Normalize a glob pattern for consistent matching.
 * - Removes trailing slashes (directories don't need them)
 */
const normalizePattern = (pattern: string): string => pattern.replace(/\/$/, "");

/**
 * Get or create a cached matcher for a pattern.
 * Handles empty pattern specially (only matches empty string).
 */
const getCachedMatcher = (pattern: string): ((path: string) => boolean) => {
  // Handle empty pattern - only matches empty string
  if (pattern === "") {
    return (path: string) => path === "";
  }

  // Normalize pattern (remove trailing slash)
  const normalizedPattern = normalizePattern(pattern);

  let matcher = matcherCache.get(normalizedPattern);
  if (!matcher) {
    matcher = picomatch(normalizedPattern, MATCH_OPTIONS);
    matcherCache.set(normalizedPattern, matcher);
  }
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
 * Only handles single brace expansion (not nested).
 *
 * @example
 * ```ts
 * expandBraces("*.{ts,tsx}"); // ["*.ts", "*.tsx"]
 * expandBraces("*.ts");       // ["*.ts"]
 * ```
 */
export const expandBraces = (pattern: string): readonly string[] => {
  const match = pattern.match(/\{([^}]+)\}/);
  if (!match?.[1]) return [pattern];
  const options = match[1].split(",");
  return options.map((opt) => pattern.replace(match[0], opt));
};

/**
 * Match a name against a pattern with brace expansion support.
 * Useful for patterns like `*.{ts,tsx}` in layout definitions.
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
  return normalized === "" ? 0 : normalized.split("/").length;
};

/**
 * Join path segments, filtering out empty strings.
 *
 * @example
 * ```ts
 * joinPath("src", "utils");  // "src/utils"
 * joinPath("", "file.ts");   // "file.ts"
 * ```
 */
export const joinPath = (...parts: readonly string[]): string =>
  parts.filter(Boolean).join("/");

/**
 * Normalize unicode strings to NFC form for consistent comparison.
 * Ensures that characters like é (composed) and e + ́ (decomposed) are treated identically.
 */
export const normalizeUnicode = (s: string): string => s.normalize("NFC");

/**
 * Clear the matcher cache. Useful for testing or when patterns change.
 */
export const clearMatcherCache = (): void => {
  matcherCache.clear();
};
