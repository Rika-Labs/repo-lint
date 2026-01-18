import { Effect, Option, Array as A } from "effect";
import picomatch from "picomatch";

export type Matcher = (path: string) => boolean;

/**
 * Picomatch options for path matching.
 * - dot: true - match dotfiles (.gitignore, .env, etc.)
 * - bash: false - ensure * doesn't match across path separators (use ** for that)
 */
const MATCH_OPTIONS = { dot: true, bash: false } as const;

export const createMatcher = (patterns: string | readonly string[]): Matcher => {
  const list = Array.isArray(patterns) ? patterns : [patterns];
  if (list.length === 0) return () => false;
  const matchers = list.map((p) => picomatch(p, MATCH_OPTIONS));
  return (path: string) => matchers.some((m) => m(path));
};

export const matches = (path: string, pattern: string): boolean =>
  picomatch(pattern, MATCH_OPTIONS)(path);

export const matchesEffect = (path: string, pattern: string): Effect.Effect<boolean> =>
  Effect.succeed(matches(path, pattern));

export const matchesAny = (path: string, patterns: readonly string[]): boolean =>
  patterns.length > 0 && createMatcher(patterns)(path);

export const matchesAnyEffect = (
  path: string,
  patterns: readonly string[],
): Effect.Effect<boolean> => Effect.succeed(matchesAny(path, patterns));

export const expandBraces = (pattern: string): readonly string[] => {
  const match = pattern.match(/\{([^}]+)\}/);
  if (!match?.[1]) return [pattern];
  const options = match[1].split(",");
  return options.map((opt) => pattern.replace(match[0], opt));
};

export const matchesWithBraces = (name: string, pattern: string): boolean => {
  const expanded = expandBraces(pattern);
  return expanded.some((p) => picomatch(p, MATCH_OPTIONS)(name));
};

export const normalizePath = (p: string): string =>
  p.replace(/\\/g, "/").replace(/\/+/g, "/").replace(/\/$/, "");

export const getBasename = (p: string): string => {
  const parts = p.split("/");
  return A.last(parts).pipe(Option.getOrElse(() => ""));
};

export const getParent = (p: string): string => {
  const parts = p.split("/");
  return parts.slice(0, -1).join("/");
};

export const getDepth = (p: string): number => (p === "" ? 0 : p.split("/").length);

export const joinPath = (...parts: readonly string[]): string =>
  parts.filter(Boolean).join("/");

/**
 * Normalize unicode strings to NFC form for consistent comparison
 */
export const normalizeUnicode = (s: string): string => s.normalize("NFC");
