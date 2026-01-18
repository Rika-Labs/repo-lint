import { Effect, Option } from "effect";
import { dirname, resolve, isAbsolute, relative } from "node:path";
import { findConfig, loadConfig, findWorkspaceConfigs } from "../config/index.js";
import { scan, scanWorkspaces, readFileContent } from "../core/scanner.js";
import { check } from "../rules/index.js";
import type { OutputFormat } from "../output/index.js";
import { readCache, writeCache, computeFileHash } from "../cache/index.js";
import type { CheckResult, RepoLintConfig } from "../types/index.js";
import { ConfigNotFoundError, ConfigParseError, ScanError, PathTraversalError } from "../errors.js";

export type ScanOverrides = {
  readonly maxDepth: Option.Option<number>;
  readonly maxFiles: Option.Option<number>;
  readonly timeoutMs: Option.Option<number>;
  readonly concurrency: Option.Option<number>;
  readonly followSymlinks: Option.Option<boolean>;
  readonly useGitignore: Option.Option<boolean>;
};

export type CheckOptions = {
  readonly scope: Option.Option<string>;
  readonly format: OutputFormat;
  readonly configPath: Option.Option<string>;
  readonly noCache: Option.Option<boolean>;
  readonly scanOverrides: ScanOverrides;
};

type CacheContext = {
  readonly enabled: boolean;
  readonly configContent: string;
};

const toPositive = (value: number | undefined): number | undefined => {
  if (value === undefined) return undefined;
  return value > 0 ? value : undefined;
};

const resolveOverride = <T>(
  override: Option.Option<T>,
  configValue: T | undefined,
): Option.Option<T> => (Option.isSome(override) ? override : Option.fromNullable(configValue));

const readConfigContent = (path: string): Effect.Effect<string, never, never> =>
  readFileContent(path).pipe(Effect.orElseSucceed(() => ""));

/**
 * Validate that a scope path doesn't escape the root
 */
const validateScopePath = (
  scope: string,
  root: string,
): Effect.Effect<void, PathTraversalError, never> =>
  Effect.gen(function* () {
    const resolvedScope = resolve(root, scope);
    const relativePath = relative(root, resolvedScope);

    if (relativePath.startsWith("..") || isAbsolute(relativePath)) {
      yield* Effect.fail(new PathTraversalError({ path: scope, configPath: root }));
    }
  });

/**
 * Scan and check a single workspace/root
 */
const scanAndCheck = (
  config: RepoLintConfig,
  root: string,
  scope: Option.Option<string>,
  cache: CacheContext,
  overrides: ScanOverrides,
): Effect.Effect<CheckResult, ScanError, never> =>
  Effect.gen(function* () {
    const scanConfig = config.scan;
    const files = yield* scan({
      root,
      ignore: config.ignore ?? [],
      scope,
      useGitignore: resolveOverride(overrides.useGitignore, config.useGitignore),
      maxDepth: resolveOverride(overrides.maxDepth, toPositive(scanConfig?.maxDepth)),
      maxFiles: resolveOverride(overrides.maxFiles, toPositive(scanConfig?.maxFiles)),
      followSymlinks: resolveOverride(overrides.followSymlinks, scanConfig?.followSymlinks),
      timeout: resolveOverride(overrides.timeoutMs, toPositive(scanConfig?.timeoutMs)),
      concurrency: resolveOverride(overrides.concurrency, toPositive(scanConfig?.concurrency)),
    });

    const fileHash = computeFileHash(files);

    if (cache.enabled) {
      const cached = yield* readCache(root, cache.configContent, fileHash);
      if (Option.isSome(cached)) {
        return cached.value.result;
      }
    }

    const result = yield* check(config, files);

    if (cache.enabled) {
      yield* writeCache(root, cache.configContent, fileHash, files.length, result);
    }

    return result;
  });

/**
 * Merge multiple check results into one
 */
const mergeResults = (results: readonly CheckResult[]): CheckResult => ({
  violations: results.flatMap((r) => [...r.violations]),
  summary: {
    total: results.reduce((a, r) => a + r.summary.total, 0),
    errors: results.reduce((a, r) => a + r.summary.errors, 0),
    warnings: results.reduce((a, r) => a + r.summary.warnings, 0),
    filesChecked: results.reduce((a, r) => a + r.summary.filesChecked, 0),
    duration: Math.max(...results.map((r) => r.summary.duration), 0),
  },
});

const workspaceConcurrency = (
  config: RepoLintConfig,
  overrides: ScanOverrides,
): number =>
  Option.getOrElse(
    resolveOverride(overrides.concurrency, toPositive(config.scan?.concurrency)),
    () => 4,
  );

export const runCheck = (
  options: CheckOptions,
): Effect.Effect<CheckResult, ConfigNotFoundError | ConfigParseError | ScanError | PathTraversalError, never> =>
  Effect.gen(function* () {
    const root = process.cwd();

    // Validate scope if provided
    if (Option.isSome(options.scope)) {
      yield* validateScopePath(options.scope.value, root);
    }

    const configPath = Option.isSome(options.configPath)
      ? Option.some(options.configPath.value)
      : yield* findConfig(root);

    if (Option.isNone(configPath)) {
      return yield* Effect.fail(new ConfigNotFoundError({ path: root }));
    }

    const config = yield* loadConfig(configPath.value).pipe(
      Effect.mapError((e) => {
        if (e._tag === "ConfigNotFoundError" || e._tag === "ConfigParseError") {
          return e;
        }
        return new ConfigParseError({ path: configPath.value, cause: e });
      }),
    );

    const configRoot = dirname(resolve(configPath.value));
    const useCache = !Option.getOrElse(options.noCache, () => false);

    const configContent = yield* readConfigContent(configPath.value);
    const cacheContext: CacheContext = { enabled: useCache, configContent };

    // Check for workspaces
    const workspaces = config.workspaces;
    let result: CheckResult;

    if (workspaces !== undefined && workspaces.length > 0) {
      const workspaceConfigs = yield* findWorkspaceConfigs(configRoot, workspaces).pipe(
        Effect.orElseSucceed(() => [] as readonly string[]),
      );

      if (workspaceConfigs.length > 0) {
        const results = yield* Effect.forEach(
          workspaceConfigs,
          (wsConfigPath) =>
            Effect.gen(function* () {
              const wsConfig = yield* loadConfig(wsConfigPath).pipe(
                Effect.mapError((e) => {
                  if (e._tag === "ConfigNotFoundError" || e._tag === "ConfigParseError") {
                    return e;
                  }
                  return new ConfigParseError({ path: wsConfigPath, cause: e });
                }),
              );
              const wsRoot = dirname(resolve(wsConfigPath));
              const wsConfigContent = yield* readConfigContent(wsConfigPath);
              return yield* scanAndCheck(wsConfig, wsRoot, options.scope, {
                enabled: useCache,
                configContent: wsConfigContent,
              }, options.scanOverrides);
            }),
          { concurrency: workspaceConcurrency(config, options.scanOverrides) },
        );
        result = mergeResults(results);
      } else {
        const workspacePaths = yield* scanWorkspaces(configRoot, workspaces).pipe(
          Effect.orElseSucceed(() => [] as readonly string[]),
        );
        const results = yield* Effect.forEach(
          workspacePaths,
          (wsPath) => scanAndCheck(config, wsPath, options.scope, cacheContext, options.scanOverrides),
          { concurrency: workspaceConcurrency(config, options.scanOverrides) },
        );
        result = mergeResults(results);
      }
    } else {
      result = yield* scanAndCheck(
        config,
        configRoot,
        options.scope,
        cacheContext,
        options.scanOverrides,
      );
    }

    return result;
  });
