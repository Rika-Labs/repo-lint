import { Effect, Option, Ref, Duration } from "effect";
import { readdir, realpath } from "node:fs/promises";
import { join, relative, resolve } from "node:path";
import type { FileEntry } from "../types/index.js";
import {
  FileSystemError,
  ScanError,
  SymlinkLoopError,
  MaxDepthExceededError,
  MaxFilesExceededError,
} from "../errors.js";
import { createMatcher, normalizePath, normalizeUnicode, joinPath } from "./matcher.js";

interface Dirent {
  name: string;
  isDirectory(): boolean;
  isFile(): boolean;
  isSymbolicLink(): boolean;
}

export type ScanOptions = {
  readonly root: string;
  readonly ignore: readonly string[];
  readonly scope: Option.Option<string>;
  readonly useGitignore: Option.Option<boolean>;
  readonly maxDepth: Option.Option<number>;
  readonly maxFiles: Option.Option<number>;
  readonly followSymlinks: Option.Option<boolean>;
  readonly timeout: Option.Option<number>;
  readonly concurrency: Option.Option<number>;
};

// Configuration constants - can be overridden via options
export const ScanDefaults = {
  MAX_DEPTH: 100,
  MAX_FILES: 100000,
  CONCURRENCY_LIMIT: 10,
  TIMEOUT_MS: 30000, // 30 seconds
} as const;

/**
 * Parse .gitignore file with basic edge case handling.
 * Patterns are scoped to the directory containing the .gitignore.
 */
const parseGitignore = (content: string, baseRel: string): readonly string[] => {
  const lines = content.split(/\r?\n/); // Handle Windows line endings
  const patterns: string[] = [];

  for (const rawLine of lines) {
    // Remove trailing spaces (but not escaped ones)
    let line = rawLine.replace(/(?<!\\)\s+$/, "");

    // Skip empty lines and comments
    if (!line || line.startsWith("#")) continue;

    // Skip negation patterns (we don't support them)
    if (line.startsWith("!")) continue;

    // Handle escaped characters
    line = line.replace(/\\#/g, "#").replace(/\\!/g, "!");

    // Directory-only patterns (ending with /)
    const isDirectoryOnly = line.endsWith("/");
    if (isDirectoryOnly) {
      line = line.slice(0, -1);
    }

    const hasSlash = line.startsWith("/") || line.includes("/");
    const normalized = line.startsWith("/") ? line.slice(1) : line;

    // Convert to glob pattern relative to base directory
    const basePattern = hasSlash
      ? joinPath(baseRel, normalized)
      : joinPath(baseRel, "**", normalized);

    patterns.push(basePattern);

    // For directory patterns, also match contents
    if (isDirectoryOnly) {
      patterns.push(`${basePattern}/**`);
    }
  }

  return patterns;
};

const readGitignoreFile = (
  dir: string,
  root: string,
): Effect.Effect<readonly string[], never, never> =>
  Effect.tryPromise({
    try: async () => {
      const gitignorePath = join(dir, ".gitignore");
      const file = Bun.file(gitignorePath);
      const exists = await file.exists();
      if (!exists) return [] as readonly string[];
      const content = await file.text();
      const baseRel = normalizePath(relative(root, dir));
      return parseGitignore(content, baseRel);
    },
    catch: () => [] as readonly string[],
  }).pipe(Effect.orElseSucceed(() => [] as readonly string[]));

const readDirectory = (
  dir: string,
): Effect.Effect<readonly Dirent[], FileSystemError, never> =>
  Effect.tryPromise({
    try: () => readdir(dir, { withFileTypes: true }) as Promise<Dirent[]>,
    catch: (cause) => new FileSystemError({ path: dir, operation: "readdir", cause }),
  });

const resolveSymlink = (
  path: string,
): Effect.Effect<string, FileSystemError, never> =>
  Effect.tryPromise({
    try: () => realpath(path),
    catch: (cause) => new FileSystemError({ path, operation: "realpath", cause }),
  });

const getFileMeta = (path: string): { size?: number; mtimeMs?: number } => {
  try {
    const file = Bun.file(path);
    return { size: file.size, mtimeMs: file.lastModified };
  } catch {
    return {};
  }
};

type ScanContext = {
  readonly root: string;
  readonly maxDepth: number;
  readonly maxFiles: number;
  readonly followSymlinks: boolean;
  readonly concurrency: number;
  readonly useGitignore: boolean;
  readonly matcherCache: Map<string, (p: string) => boolean>;
  readonly visitedPaths: Set<string>;
  readonly fileCount: Ref.Ref<number>;
};

const matcherFor = (
  ctx: ScanContext,
  patterns: readonly string[],
): ((p: string) => boolean) => {
  const key = patterns.join("\n");
  const cached = ctx.matcherCache.get(key);
  if (cached) return cached;
  const matcher = createMatcher(patterns);
  ctx.matcherCache.set(key, matcher);
  return matcher;
};

const scanDirectory = (
  dir: string,
  ctx: ScanContext,
  currentDepth: number,
  inheritedPatterns: readonly string[],
): Effect.Effect<
  readonly FileEntry[],
  FileSystemError | SymlinkLoopError | MaxDepthExceededError | MaxFilesExceededError,
  never
> =>
  Effect.gen(function* () {
    const rel = normalizePath(relative(ctx.root, dir));

    // Read local .gitignore if enabled
    const localPatterns = ctx.useGitignore
      ? [...inheritedPatterns, ...(yield* readGitignoreFile(dir, ctx.root))]
      : inheritedPatterns;

    const ignoreMatcher = matcherFor(ctx, localPatterns);

    if (rel && ignoreMatcher(rel)) return [];

    const entries = yield* readDirectory(dir).pipe(
      Effect.catchAll((error) => {
        const errorStr = String(error.cause);
        if (errorStr.includes("EACCES") || errorStr.includes("EPERM") || errorStr.includes("ENOENT")) {
          return Effect.succeed([] as readonly Dirent[]);
        }
        return Effect.fail(error);
      }),
    );

    const results: FileEntry[] = [];
    const subdirs: Array<{ path: string; depth: number; patterns: readonly string[] }> = [];

    for (const entry of entries) {
      // Skip files with problematic names
      if (entry.name.includes("\0")) continue;

      const fullPath = join(dir, entry.name);
      const normalizedName = normalizeUnicode(entry.name);
      const relPath = normalizePath(relative(ctx.root, join(dir, normalizedName)));

      if (ignoreMatcher(relPath)) continue;

      // Check max files
      const count = yield* Ref.get(ctx.fileCount);
      if (count >= ctx.maxFiles) {
        yield* Effect.fail(new MaxFilesExceededError({ count, maxFiles: ctx.maxFiles }));
      }
      yield* Ref.update(ctx.fileCount, (n) => n + 1);

      const depth = currentDepth + 1;
      const isLink = entry.isSymbolicLink();

      if (isLink && ctx.followSymlinks) {
        const targetResult = yield* resolveSymlink(fullPath).pipe(
          Effect.map(Option.some),
          Effect.catchAll(() => Effect.succeed(Option.none<string>())),
        );

        if (Option.isSome(targetResult)) {
          const target = targetResult.value;
          const resolvedTarget = resolve(target);

          if (ctx.visitedPaths.has(resolvedTarget)) {
            yield* Effect.fail(new SymlinkLoopError({ path: fullPath, target }));
          }
          ctx.visitedPaths.add(resolvedTarget);
        }
      }

      if (entry.isDirectory() || (isLink && ctx.followSymlinks)) {
        results.push({
          path: fullPath,
          relativePath: relPath,
          isDirectory: true,
          isSymlink: isLink,
          depth,
        });
        if (currentDepth < ctx.maxDepth) {
          subdirs.push({ path: fullPath, depth, patterns: localPatterns });
        } else {
          // We're at maxDepth and found a subdirectory - this exceeds the limit
          yield* Effect.fail(
            new MaxDepthExceededError({ path: fullPath, depth, maxDepth: ctx.maxDepth })
          );
        }
      } else if (entry.isFile()) {
        const meta = getFileMeta(fullPath);
        results.push({
          path: fullPath,
          relativePath: relPath,
          isDirectory: false,
          isSymlink: isLink,
          depth,
          ...meta,
        });
      }
    }

    const subdirResults = yield* Effect.forEach(
      subdirs,
      ({ path, depth, patterns }) => scanDirectory(path, ctx, depth, patterns),
      { concurrency: ctx.concurrency },
    );

    return [...results, ...subdirResults.flat()];
  });

export const scan = (options: ScanOptions): Effect.Effect<readonly FileEntry[], ScanError, never> =>
  Effect.gen(function* () {
    const scopePath = Option.getOrElse(options.scope, () => "");
    const root = scopePath ? join(options.root, scopePath) : options.root;
    const ignorePatterns = [...options.ignore];

    const ctx: ScanContext = {
      root,
      maxDepth: Option.getOrElse(options.maxDepth, () => ScanDefaults.MAX_DEPTH),
      maxFiles: Option.getOrElse(options.maxFiles, () => ScanDefaults.MAX_FILES),
      followSymlinks: Option.getOrElse(options.followSymlinks, () => false),
      concurrency: Option.getOrElse(options.concurrency, () => ScanDefaults.CONCURRENCY_LIMIT),
      useGitignore: Option.getOrElse(options.useGitignore, () => true),
      matcherCache: new Map(),
      visitedPaths: new Set([resolve(root)]),
      fileCount: yield* Ref.make(0),
    };

    const timeoutMs = Option.getOrElse(options.timeout, () => ScanDefaults.TIMEOUT_MS);

    return yield* scanDirectory(root, ctx, scopePath ? scopePath.split("/").length : 0, ignorePatterns).pipe(
      Effect.timeoutFail({
        duration: Duration.millis(timeoutMs),
        onTimeout: () => new ScanError({ root, cause: "Scan timed out" }),
      }),
      Effect.mapError((cause) =>
        cause instanceof ScanError ? cause : new ScanError({ root, cause }),
      ),
    );
  });

export const scanWorkspaces = (
  root: string,
  patterns: readonly string[],
): Effect.Effect<readonly string[], FileSystemError, never> =>
  Effect.gen(function* () {
    const results: string[] = [];
    const matcher = createMatcher(patterns);

    const entries = yield* readDirectory(root).pipe(
      Effect.catchAll(() => Effect.succeed([] as readonly Dirent[])),
    );

    for (const entry of entries) {
      if (entry.isDirectory() && matcher(entry.name)) {
        results.push(join(root, entry.name));
      }
    }

    for (const pattern of patterns) {
      if (pattern.includes("/")) {
        const parts = pattern.split("/");
        const baseDir = parts[0];
        if (!baseDir) continue;

        const basePath = join(root, baseDir);

        const subEntries = yield* readDirectory(basePath).pipe(
          Effect.catchAll(() => Effect.succeed([] as readonly Dirent[])),
        );

        for (const entry of subEntries) {
          if (entry.isDirectory()) {
            const relPath = `${baseDir}/${entry.name}`;
            if (matcher(relPath)) {
              results.push(join(root, relPath));
            }
          }
        }
      }
    }

    return [...new Set(results)];
  });

export const readFileContent = (path: string): Effect.Effect<string, FileSystemError, never> =>
  Effect.tryPromise({
    try: () => Bun.file(path).text(),
    catch: (cause) => new FileSystemError({ path, operation: "read", cause }),
  });

export const fileExists = (path: string): Effect.Effect<boolean, never, never> =>
  Effect.tryPromise({
    try: () => Bun.file(path).exists(),
    catch: () => false,
  }).pipe(Effect.orElseSucceed(() => false));
