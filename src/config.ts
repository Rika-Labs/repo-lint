import { Effect, Option } from "effect";
import { join, dirname, resolve } from "node:path";
import YAML from "yaml";
import type { RepoLintConfig, Rules, BoundaryRule } from "./types.js";
import { ConfigNotFoundError, ConfigParseError, FileSystemError } from "./errors.js";
import { readFileContent, fileExists } from "./scanner.js";
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
    while (dir !== "/") {
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

export const loadConfig = (
  configPath: string,
): Effect.Effect<RepoLintConfig, ConfigParseError | ConfigNotFoundError, never> =>
  Effect.gen(function* () {
    const content = yield* readFileContent(configPath).pipe(
      Effect.mapError(() => new ConfigNotFoundError({ path: configPath })),
    );

    const config = yield* Effect.try({
      try: () => YAML.parse(content) as RepoLintConfig,
      catch: (cause) => new ConfigParseError({ path: configPath, cause }),
    });

    if (config.extends !== undefined) {
      const configDir = dirname(configPath);
      const repoRoot = yield* findRepoRoot(configDir);

      const basePath = config.extends.startsWith("@/")
        ? resolve(repoRoot, config.extends.slice(2))
        : resolve(configDir, config.extends);

      const baseConfig = yield* loadConfig(basePath);
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

export const loadConfigFromRoot = (
  root: string,
): Effect.Effect<RepoLintConfig, ConfigNotFoundError | ConfigParseError, never> =>
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
      }).pipe(Effect.orElseSucceed(() => []));

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
