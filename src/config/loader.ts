import { Effect, Option } from "effect";
import { Schema, TreeFormatter, ParseResult } from "@effect/schema";
import { join, dirname, resolve, relative, isAbsolute } from "node:path";
import YAML from "yaml";
import type { RepoLintConfig, Rules, BoundaryRule } from "../types/index.js";
import { RepoLintConfigSchema } from "../types/index.js";
import {
  ConfigNotFoundError,
  ConfigParseError,
  ConfigValidationError,
  CircularExtendsError,
  PathTraversalError,
  FileSystemError,
} from "../errors.js";
import { readFileContent, fileExists } from "../core/scanner.js";
import { nextjsPreset } from "./presets/nextjs.js";

const CONFIG_NAMES = [
  "repo-lint.config.yaml",
  "repo-lint.config.yml",
  ".repo-lint.yaml",
  ".repo-lint.yml",
] as const;

export const findConfig = (
  root: string,
): Effect.Effect<Option.Option<string>, never, never> =>
  Effect.gen(function* () {
    for (const name of CONFIG_NAMES) {
      const path = join(root, name);
      const exists = yield* fileExists(path);
      if (exists) return Option.some(path);
    }
    return Option.none();
  });

const findRepoRoot = (from: string): Effect.Effect<string, never, never> =>
  Effect.gen(function* () {
    let dir = from;
    const root = isAbsolute("/") ? "/" : dir.split("/")[0] ?? "/";

    while (dir !== root && dir !== "") {
      const hasPackageJson = yield* fileExists(join(dir, "package.json"));
      if (hasPackageJson) return dir;

      const hasGit = yield* fileExists(join(dir, ".git"));
      if (hasGit) return dir;

      dir = dirname(dir);
    }
    return from;
  });

const getPreset = (name: string): Option.Option<RepoLintConfig> => {
  switch (name) {
    case "nextjs":
    case "nextjs-app":
      return Option.some(nextjsPreset());
    default:
      return Option.none();
  }
};

const resolveBoundaries = (
  base: BoundaryRule | undefined,
  override: BoundaryRule | undefined,
): BoundaryRule | undefined => {
  if (override !== undefined) return override;
  if (base !== undefined) return base;
  return undefined;
};

const mergeRules = (base: Rules | undefined, override: Rules | undefined): Rules => {
  const boundaries = resolveBoundaries(base?.boundaries, override?.boundaries);

  const result: Rules = {
    forbidPaths: [...(base?.forbidPaths ?? []), ...(override?.forbidPaths ?? [])],
    forbidNames: [...(base?.forbidNames ?? []), ...(override?.forbidNames ?? [])],
    ignorePaths: [...(base?.ignorePaths ?? []), ...(override?.ignorePaths ?? [])],
    dependencies: { ...base?.dependencies, ...override?.dependencies },
    mirror: [...(base?.mirror ?? []), ...(override?.mirror ?? [])],
    when: { ...base?.when, ...override?.when },
    match: [...(base?.match ?? []), ...(override?.match ?? [])],
  };

  if (boundaries !== undefined) {
    return { ...result, boundaries };
  }

  return result;
};

const mergeConfigs = (base: RepoLintConfig, override: RepoLintConfig): RepoLintConfig => {
  const result: RepoLintConfig = {
    ignore: [...(base.ignore ?? []), ...(override.ignore ?? [])],
    rules: mergeRules(base.rules, override.rules),
  };

  if (override.mode !== undefined) {
    return { ...result, mode: override.mode };
  }
  if (base.mode !== undefined) {
    return { ...result, mode: base.mode };
  }

  if (override.layout !== undefined) {
    return { ...result, layout: override.layout };
  }
  if (base.layout !== undefined) {
    return { ...result, layout: base.layout };
  }

  if (override.useGitignore !== undefined) {
    return { ...result, useGitignore: override.useGitignore };
  }
  if (base.useGitignore !== undefined) {
    return { ...result, useGitignore: base.useGitignore };
  }

  if (override.workspaces !== undefined) {
    return { ...result, workspaces: override.workspaces };
  }
  if (base.workspaces !== undefined) {
    return { ...result, workspaces: base.workspaces };
  }

  return result;
};

/**
 * Validate that extends path doesn't escape the repo root
 */
const validateExtendsPath = (
  extendsPath: string,
  configPath: string,
  repoRoot: string,
): Effect.Effect<string, PathTraversalError, never> =>
  Effect.gen(function* () {
    const configDir = dirname(configPath);

    const resolvedPath = extendsPath.startsWith("@/")
      ? resolve(repoRoot, extendsPath.slice(2))
      : resolve(configDir, extendsPath);

    const relativePath = relative(repoRoot, resolvedPath);

    // Check if path escapes repo root
    if (relativePath.startsWith("..") || isAbsolute(relativePath)) {
      yield* Effect.fail(new PathTraversalError({ path: extendsPath, configPath }));
    }

    return resolvedPath;
  });

/**
 * Format validation errors in a user-friendly way
 */
const formatValidationErrors = (error: ParseResult.ParseError): readonly string[] => {
  const errors: string[] = [];

  // Try to use TreeFormatter for structured output
  try {
    const formatted = TreeFormatter.formatErrorSync(error);
    errors.push(formatted);
  } catch {
    // Fallback to basic message
    errors.push(error.message);
  }

  return errors;
};

const collectUnknownKeys = (
  obj: Record<string, unknown>,
  allowed: ReadonlySet<string>,
  label: string,
): readonly string[] => {
  const errors: string[] = [];
  for (const key of Object.keys(obj)) {
    if (!allowed.has(key)) {
      errors.push(`Unknown ${label} key: ${key}`);
    }
  }
  return errors;
};

const CONFIG_KEYS = new Set([
  "mode",
  "extends",
  "layout",
  "ignore",
  "useGitignore",
  "workspaces",
  "rules",
  "scan",
  "preset",
]);

const RULE_KEYS = new Set([
  "forbidPaths",
  "forbidNames",
  "ignorePaths",
  "dependencies",
  "mirror",
  "when",
  "boundaries",
  "match",
]);

const SCAN_KEYS = new Set([
  "maxDepth",
  "maxFiles",
  "followSymlinks",
  "timeoutMs",
  "concurrency",
]);

/**
 * Parse and validate config against schema
 */
const parseConfig = (
  content: string,
  configPath: string,
): Effect.Effect<RepoLintConfig, ConfigParseError | ConfigValidationError, never> =>
  Effect.gen(function* () {
    // Handle empty config file
    if (!content.trim()) {
      return yield* Effect.fail(
        new ConfigValidationError({
          path: configPath,
          errors: ["Config file is empty"],
        }),
      );
    }

    // Parse YAML
    let rawConfig: unknown;
    try {
      rawConfig = YAML.parse(content);
    } catch (cause) {
      return yield* Effect.fail(new ConfigParseError({ path: configPath, cause }));
    }

    // Handle whitespace-only or null YAML
    if (rawConfig === null || rawConfig === undefined) {
      return yield* Effect.fail(
        new ConfigValidationError({
          path: configPath,
          errors: ["Config file contains no valid configuration"],
        }),
      );
    }

    // Check for unknown keys before schema validation
    if (typeof rawConfig === "object") {
      const configObj = rawConfig as Record<string, unknown>;
      const errors = [...collectUnknownKeys(configObj, CONFIG_KEYS, "config")];

      const rulesValue = configObj["rules"];
      if (rulesValue && typeof rulesValue === "object" && !Array.isArray(rulesValue)) {
        errors.push(...collectUnknownKeys(rulesValue as Record<string, unknown>, RULE_KEYS, "rules"));
      }

      const scanValue = configObj["scan"];
      if (scanValue && typeof scanValue === "object" && !Array.isArray(scanValue)) {
        errors.push(...collectUnknownKeys(scanValue as Record<string, unknown>, SCAN_KEYS, "scan"));
      }

      if (errors.length > 0) {
        return yield* Effect.fail(new ConfigValidationError({ path: configPath, errors }));
      }
    }

    // Validate against schema
    const parseResult = Schema.decodeUnknownEither(RepoLintConfigSchema)(rawConfig);

    if (parseResult._tag === "Left") {
      const errors = formatValidationErrors(parseResult.left);
      return yield* Effect.fail(new ConfigValidationError({ path: configPath, errors }));
    }

    return parseResult.right;
  });

/**
 * Load config with circular extends detection
 */
const loadConfigInternal = (
  configPath: string,
  chain: readonly string[],
): Effect.Effect<
  RepoLintConfig,
  ConfigNotFoundError | ConfigParseError | ConfigValidationError | CircularExtendsError | PathTraversalError,
  never
> =>
  Effect.gen(function* () {
    // Check for circular extends
    if (chain.includes(configPath)) {
      yield* Effect.fail(new CircularExtendsError({ path: configPath, chain }));
    }

    const content = yield* readFileContent(configPath).pipe(
      Effect.mapError(() => new ConfigNotFoundError({ path: configPath })),
    );

    const config = yield* parseConfig(content, configPath);

    if (config.extends !== undefined) {
      const configDir = dirname(configPath);
      const repoRoot = yield* findRepoRoot(configDir);

      const basePath = yield* validateExtendsPath(config.extends, configPath, repoRoot);

      const baseConfig = yield* loadConfigInternal(basePath, [...chain, configPath]);
      return mergeConfigs(baseConfig, config);
    }

    if (config.preset !== undefined) {
      const presetConfig = getPreset(config.preset);
      if (Option.isSome(presetConfig)) {
        return mergeConfigs(presetConfig.value, config);
      }
    }

    return config;
  });

export const loadConfig = (
  configPath: string,
): Effect.Effect<
  RepoLintConfig,
  ConfigNotFoundError | ConfigParseError | ConfigValidationError | CircularExtendsError | PathTraversalError,
  never
> => loadConfigInternal(configPath, []);

export const loadConfigFromRoot = (
  root: string,
): Effect.Effect<
  RepoLintConfig,
  ConfigNotFoundError | ConfigParseError | ConfigValidationError | CircularExtendsError | PathTraversalError,
  never
> =>
  Effect.gen(function* () {
    const configPath = yield* findConfig(root);

    if (Option.isNone(configPath)) {
      return yield* Effect.fail(new ConfigNotFoundError({ path: root }));
    }

    return yield* loadConfig(configPath.value);
  });

export const findWorkspaceConfigs = (
  root: string,
  workspaces: readonly string[],
): Effect.Effect<readonly string[], FileSystemError, never> =>
  Effect.gen(function* () {
    const configs: string[] = [];

    for (const pattern of workspaces) {
      const base = pattern.replace(/\/\*$/, "");
      const basePath = join(root, base);

      const exists = yield* fileExists(basePath);
      if (!exists) continue;

      const dirs = yield* Effect.tryPromise({
        try: async () => {
          const { readdir } = await import("node:fs/promises");
          return readdir(basePath, { withFileTypes: true });
        },
        catch: (cause) => new FileSystemError({ path: basePath, operation: "readdir", cause }),
      }).pipe(
        Effect.catchAll((error) => {
          // Log permission errors but continue
          if (String(error.cause).includes("EACCES")) {
            return Effect.succeed([]);
          }
          return Effect.fail(error);
        }),
      );

      for (const dir of dirs) {
        if (dir.isDirectory()) {
          const wsPath = join(basePath, dir.name);
          const wsConfig = yield* findConfig(wsPath);
          if (Option.isSome(wsConfig)) {
            configs.push(wsConfig.value);
          }
        }
      }
    }

    return configs;
  });
