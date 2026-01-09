import { Effect, Option, Duration } from "effect";
import { join } from "node:path";
import { createHash } from "node:crypto";
import type { FileEntry, CheckResult } from "../types/index.js";

/** Directory name for cache files */
const CACHE_DIR = ".repo-lint-cache";

/** Cache format version - increment when cache structure changes */
const CACHE_VERSION = "2";

/** Default cache TTL: 1 hour in milliseconds */
const DEFAULT_CACHE_TTL_MS = 60 * 60 * 1000;

type CacheEntry = {
  readonly version: string;
  readonly root: string;
  readonly configHash: string;
  readonly fileHash: string;
  readonly filesCount: number;
  readonly result: CheckResult;
  readonly timestamp: number;
};

/**
 * Create a short hash of a string for cache key comparison
 */
const hashString = (str: string): string => {
  return createHash("sha256").update(str).digest("hex").slice(0, 16);
};

/**
 * Get the path to the cache file for a given root
 */
const getCachePath = (root: string): string => join(root, CACHE_DIR, "cache.json");

/**
 * Compute a deterministic hash of the scanned file list
 */
export const computeFileHash = (files: readonly FileEntry[]): string => {
  const normalized = files
    .map((f) => {
      const kind = f.isDirectory ? "D" : "F";
      const size = f.size ?? 0;
      const mtime = f.mtimeMs ?? 0;
      return `${f.relativePath}:${kind}:${size}:${mtime}`;
    })
    .sort()
    .join("\n");
  return hashString(normalized);
};

/**
 * Validate a cache entry is still valid
 */
const validateCacheEntry = (
  entry: CacheEntry,
  root: string,
  configContent: string,
  fileHash: string,
  maxAgeMs: number = DEFAULT_CACHE_TTL_MS,
): boolean => {
  // Check cache version
  if (entry.version !== CACHE_VERSION) {
    return false;
  }

  // Check root matches
  if (entry.root !== root) {
    return false;
  }

  // Check config hash matches
  const currentHash = hashString(configContent);
  if (entry.configHash !== currentHash) {
    return false;
  }

  // Check file hash matches
  if (entry.fileHash !== fileHash) {
    return false;
  }

  // Check TTL
  if (Date.now() - entry.timestamp > maxAgeMs) {
    return false;
  }

  return true;
};

/**
 * Read cached check result if valid
 */
export const readCache = (
  root: string,
  configContent: string,
  fileHash: string,
  maxAgeMs: number = DEFAULT_CACHE_TTL_MS,
): Effect.Effect<Option.Option<CacheEntry>, never, never> =>
  Effect.tryPromise({
    try: async () => {
      const cachePath = getCachePath(root);
      const content = await Bun.file(cachePath).text();
      const entry = JSON.parse(content) as CacheEntry;

      if (!validateCacheEntry(entry, root, configContent, fileHash, maxAgeMs)) {
        return Option.none<CacheEntry>();
      }

      return Option.some(entry);
    },
    catch: () => Option.none<CacheEntry>(),
  }).pipe(
    Effect.timeout(Duration.millis(1000)),
    Effect.catchAll(() => Effect.succeed(Option.none<CacheEntry>())),
  );

/**
 * Write check result to cache
 */
export const writeCache = (
  root: string,
  configContent: string,
  fileHash: string,
  filesCount: number,
  result: CheckResult,
): Effect.Effect<void, never, never> =>
  Effect.tryPromise({
    try: async () => {
      const cachePath = getCachePath(root);
      const entry: CacheEntry = {
        version: CACHE_VERSION,
        root,
        configHash: hashString(configContent),
        fileHash,
        filesCount,
        result,
        timestamp: Date.now(),
      };

      // Ensure cache directory exists
      const { mkdir } = await import("node:fs/promises");
      await mkdir(join(root, CACHE_DIR), { recursive: true });

      await Bun.write(cachePath, JSON.stringify(entry));
    },
    catch: () => undefined,
  }).pipe(
    Effect.timeout(Duration.millis(1000)),
    Effect.catchAll(() => Effect.succeed(undefined)),
    Effect.asVoid,
  );

/**
 * Clear all cached data for a root
 */
export const clearCache = (root: string): Effect.Effect<void, never, never> =>
  Effect.tryPromise({
    try: async () => {
      const { rm } = await import("node:fs/promises");
      await rm(join(root, CACHE_DIR), { recursive: true, force: true });
    },
    catch: () => undefined,
  }).pipe(
    Effect.catchAll(() => Effect.succeed(undefined)),
    Effect.asVoid,
  );

/**
 * Get cache statistics (for debugging)
 */
export const getCacheStats = (
  root: string,
): Effect.Effect<Option.Option<{ age: number; size: number; filesCount: number }>, never, never> =>
  Effect.tryPromise({
    try: async () => {
      const cachePath = getCachePath(root);
      const file = Bun.file(cachePath);
      const exists = await file.exists();

      if (!exists) {
        return Option.none<{ age: number; size: number; filesCount: number }>();
      }

      const content = await file.text();
      const entry = JSON.parse(content) as CacheEntry;

      return Option.some({
        age: Date.now() - entry.timestamp,
        size: content.length,
        filesCount: entry.filesCount,
      });
    },
    catch: () => Option.none<{ age: number; size: number; filesCount: number }>(),
  }).pipe(Effect.orElseSucceed(() => Option.none<{ age: number; size: number; filesCount: number }>()));
