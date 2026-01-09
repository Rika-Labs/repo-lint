#!/usr/bin/env bun
import { Effect, Option, Console } from "effect";
import { resolve, dirname } from "node:path";
import { findConfig, loadConfig, findWorkspaceConfigs } from "./config.js";
import { scan, scanWorkspaces } from "./scanner.js";
import { check } from "./checker.js";
import { format, type OutputFormat } from "./output.js";
import type { CheckResult } from "./types.js";
import { ConfigNotFoundError, ConfigParseError, ScanError } from "./errors.js";

const VERSION = "0.5.0";

const HELP = `
repo-lint v${VERSION} - High-performance filesystem layout linter

USAGE:
  repo-lint check [options]
  repo-lint inspect <subcommand>

COMMANDS:
  check              Check filesystem against config (default)
  inspect layout     Print resolved layout tree
  inspect path <p>   Check if path is allowed
  inspect rule <r>   Get rule details

CHECK OPTIONS:
  --scope <path>     Only validate a subtree
  --json             Output as JSON
  --sarif            Output as SARIF (GitHub Code Scanning)
  --config <file>    Use specific config file

GLOBAL OPTIONS:
  --help, -h         Show this help
  --version, -v      Show version
`;

type Args = {
  readonly command: "check" | "inspect" | "help" | "version";
  readonly scope: string | undefined;
  readonly format: OutputFormat;
  readonly config: string | undefined;
  readonly inspectType: "layout" | "path" | "rule" | undefined;
  readonly inspectArg: string | undefined;
};

const parseArgs = (argv: readonly string[]): Args => {
  let command: Args["command"] = "check";
  let scope: string | undefined;
  let fmt: OutputFormat = "console";
  let config: string | undefined;
  let inspectType: Args["inspectType"];
  let inspectArg: string | undefined;

  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];

    if (arg === "check") {
      command = "check";
    } else if (arg === "inspect") {
      command = "inspect";
      const next = argv[i + 1];
      if (next === "layout" || next === "path" || next === "rule") {
        inspectType = next;
        i++;
        if (next !== "layout") {
          const nextArg = argv[i + 1];
          if (nextArg !== undefined) {
            inspectArg = nextArg;
            i++;
          }
        }
      }
    } else if (arg === "--scope") {
      const nextArg = argv[i + 1];
      if (nextArg !== undefined) {
        scope = nextArg;
        i++;
      }
    } else if (arg === "--json") {
      fmt = "json";
    } else if (arg === "--sarif") {
      fmt = "sarif";
    } else if (arg === "--config") {
      const nextArg = argv[i + 1];
      if (nextArg !== undefined) {
        config = nextArg;
        i++;
      }
    } else if (arg === "--help" || arg === "-h") {
      command = "help";
    } else if (arg === "--version" || arg === "-v") {
      command = "version";
    }
  }

  return { command, scope, format: fmt, config, inspectType, inspectArg };
};

const runCheck = (args: Args): Effect.Effect<void, ConfigNotFoundError | ConfigParseError | ScanError> =>
  Effect.gen(function* () {
    const root = process.cwd();

    const configPath = args.config !== undefined
      ? Option.some(args.config)
      : yield* findConfig(root);

    if (Option.isNone(configPath)) {
      yield* Console.error("No config file found. Create repo-lint.config.yaml");
      return yield* Effect.fail(new ConfigNotFoundError({ path: root }));
    }

    const config = yield* loadConfig(configPath.value);
    const configRoot = dirname(resolve(configPath.value));

    const results: CheckResult[] = [];

    const workspaces = config.workspaces;
    if (workspaces !== undefined && workspaces.length > 0) {
      const workspaceConfigs = yield* findWorkspaceConfigs(configRoot, workspaces).pipe(
        Effect.orElseSucceed(() => [] as readonly string[]),
      );

      if (workspaceConfigs.length > 0) {
        for (const wsConfigPath of workspaceConfigs) {
          const wsConfig = yield* loadConfig(wsConfigPath);
          const wsRoot = dirname(resolve(wsConfigPath));
          const files = yield* scan({
            root: wsRoot,
            ignore: wsConfig.ignore ?? [],
            scope: args.scope,
            useGitignore: wsConfig.useGitignore,
          });
          const result = yield* check(wsConfig, files);
          results.push(result);
        }
      } else {
        const workspacePaths = yield* scanWorkspaces(configRoot, workspaces).pipe(
          Effect.orElseSucceed(() => [] as readonly string[]),
        );

        for (const wsPath of workspacePaths) {
          const files = yield* scan({
            root: wsPath,
            ignore: config.ignore ?? [],
            scope: args.scope,
            useGitignore: config.useGitignore,
          });
          const result = yield* check(config, files);
          results.push(result);
        }
      }
    } else {
      const files = yield* scan({
        root: configRoot,
        ignore: config.ignore ?? [],
        scope: args.scope,
        useGitignore: config.useGitignore,
      });
      const result = yield* check(config, files);
      results.push(result);
    }

    const merged: CheckResult = {
      violations: results.flatMap((r) => r.violations),
      summary: {
        total: results.reduce((a, r) => a + r.summary.total, 0),
        errors: results.reduce((a, r) => a + r.summary.errors, 0),
        warnings: results.reduce((a, r) => a + r.summary.warnings, 0),
        filesChecked: results.reduce((a, r) => a + r.summary.filesChecked, 0),
        duration: results.reduce((a, r) => a + r.summary.duration, 0),
      },
    };

    yield* Console.log(format(merged, args.format));

    if (merged.summary.errors > 0) {
      process.exit(1);
    }

    return;
  });

const runInspect = (args: Args): Effect.Effect<void, ConfigNotFoundError | ConfigParseError> =>
  Effect.gen(function* () {
    const root = process.cwd();
    const configPath = args.config !== undefined ? Option.some(args.config) : yield* findConfig(root);

    if (Option.isNone(configPath)) {
      yield* Console.error("No config file found");
      return yield* Effect.fail(new ConfigNotFoundError({ path: root }));
    }

    const config = yield* loadConfig(configPath.value);

    switch (args.inspectType) {
      case "layout":
        yield* Console.log(JSON.stringify(config.layout, null, 2));
        return;
      case "path":
        if (args.inspectArg === undefined) {
          yield* Console.error("Usage: repo-lint inspect path <path>");
          process.exit(1);
        }
        yield* Console.log(`Path: ${args.inspectArg}`);
        yield* Console.log("(Path inspection not yet implemented)");
        return;
      case "rule":
        if (args.inspectArg === undefined) {
          yield* Console.error("Usage: repo-lint inspect rule <rule>");
          process.exit(1);
        }
        const rules = config.rules ?? {};
        const rule = (rules as Record<string, unknown>)[args.inspectArg];
        if (rule !== undefined) {
          yield* Console.log(JSON.stringify(rule, null, 2));
        } else {
          yield* Console.log(`Rule "${args.inspectArg}" not found`);
        }
        return;
      default:
        yield* Console.log("Usage: repo-lint inspect <layout|path|rule>");
        return;
    }
  });

const program = Effect.gen(function* () {
  const args = parseArgs(process.argv.slice(2));

  switch (args.command) {
    case "help":
      yield* Console.log(HELP);
      return;
    case "version":
      yield* Console.log(VERSION);
      return;
    case "inspect":
      yield* runInspect(args);
      return;
    case "check":
      yield* runCheck(args);
      return;
  }
});

Effect.runPromise(program).catch((e: unknown) => {
  const message = e instanceof Error ? e.message : String(e);
  console.error(message);
  process.exit(1);
});
