import { Effect } from "effect";
import { readdir } from "node:fs/promises";
import { join, relative } from "node:path";
import type { FileEntry } from "./types.js";
import { FileSystemError, ScanError } from "./errors.js";
import { createMatcher, normalizePath } from "./matcher.js";

interface Dirent {
  name: string;
  isDirectory(): boolean;
  isFile(): boolean;
}

export type ScanOptions = {
  readonly root: string;
  readonly ignore: readonly string[];
  readonly scope?: string | undefined;
  readonly useGitignore?: boolean | undefined;
};

const readGitignore = (root: string): Effect.Effect<readonly string[], never, never> =>
  Effect.tryPromise({
    try: async () => {
      const content = await Bun.file(join(root, ".gitignore")).text();
      return content
        .split("\n")
        .map((l) => l.trim())
        .filter((l) => l && !l.startsWith("#"))
        .map((l) => (l.startsWith("/") ? l.slice(1) : `**/${l}`));
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

const scanDirectory = (
  dir: string,
  root: string,
  ignoreMatcher: (p: string) => boolean,
  baseDepth: number,
): Effect.Effect<readonly FileEntry[], FileSystemError, never> =>
  Effect.gen(function* () {
    const rel = normalizePath(relative(root, dir));
    if (rel && ignoreMatcher(rel)) return [];

    const entries = yield* readDirectory(dir).pipe(
      Effect.orElseSucceed(() => [] as readonly Dirent[]),
    );

    const results: FileEntry[] = [];
    const subdirs: string[] = [];

    for (const entry of entries) {
      const fullPath = join(dir, entry.name);
      const relPath = normalizePath(relative(root, fullPath));

      if (ignoreMatcher(relPath)) continue;

      const depth = baseDepth + relPath.split("/").length;

      if (entry.isDirectory()) {
        results.push({ path: fullPath, relativePath: relPath, isDirectory: true, depth });
        subdirs.push(fullPath);
      } else if (entry.isFile()) {
        results.push({ path: fullPath, relativePath: relPath, isDirectory: false, depth });
      }
    }

    const subdirResults = yield* Effect.forEach(
      subdirs,
      (d) => scanDirectory(d, root, ignoreMatcher, baseDepth),
      { concurrency: "unbounded" },
    );

    return [...results, ...subdirResults.flat()];
  });

export const scan = (
  options: ScanOptions,
): Effect.Effect<readonly FileEntry[], ScanError, never> =>
  Effect.gen(function* () {
    const root = options.scope !== undefined ? join(options.root, options.scope) : options.root;
    const ignorePatterns = [...options.ignore];

    if (options.useGitignore !== false) {
      const gitignore = yield* readGitignore(options.root);
      ignorePatterns.push(...gitignore);
    }

    const ignoreMatcher = createMatcher(ignorePatterns);
    const baseDepth = options.scope !== undefined ? options.scope.split("/").length : 0;

    return yield* scanDirectory(root, root, ignoreMatcher, baseDepth).pipe(
      Effect.mapError((cause) => new ScanError({ root, cause })),
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
      Effect.orElseSucceed(() => [] as readonly Dirent[]),
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
          Effect.orElseSucceed(() => [] as readonly Dirent[]),
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

export const readFileContent = (
  path: string,
): Effect.Effect<string, FileSystemError, never> =>
  Effect.tryPromise({
    try: () => Bun.file(path).text(),
    catch: (cause) => new FileSystemError({ path, operation: "read", cause }),
  });

export const fileExists = (path: string): Effect.Effect<boolean, never, never> =>
  Effect.tryPromise({
    try: () => Bun.file(path).exists(),
    catch: () => false,
  }).pipe(Effect.orElseSucceed(() => false));
