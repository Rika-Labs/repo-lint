#!/usr/bin/env bun
import { Effect, Option, Console, Exit, Cause } from "effect";
import { parseArgs } from "./parser.js";
import { runCheck } from "../commands/check.js";
import { runInspect } from "../commands/inspect.js";
import { getVersion } from "../version.js";
import { format } from "../output/index.js";

const HELP = `
repo-lint v${getVersion()} - High-performance filesystem layout linter

USAGE:
  repo-lint check [options]
  repo-lint inspect <subcommand>

COMMANDS:
  check              Check filesystem against config (default)
  inspect layout     Print resolved layout tree
  inspect rule <r>   Get rule details

CHECK OPTIONS:
  --scope <path>        Only validate a subtree
  --json                Output as JSON
  --sarif               Output as SARIF (GitHub Code Scanning)
  --config <file>       Use specific config file
  --no-cache            Disable result caching
  --max-depth <n>       Override scan max depth
  --max-files <n>       Override scan max files
  --timeout-ms <n>      Override scan timeout (ms)
  --concurrency <n>     Override scan concurrency
  --follow-symlinks     Follow symlinks during scan
  --no-gitignore        Disable .gitignore processing

GLOBAL OPTIONS:
  --help, -h            Show this help
  --version, -v         Show version
`;

const program = Effect.gen(function* () {
  const args = parseArgs(process.argv.slice(2));

  if (args.errors.length > 0) {
    for (const error of args.errors) {
      yield* Console.error(error);
    }
    return 1;
  }

  switch (args.command) {
    case "help":
      yield* Console.log(HELP);
      return 0;

    case "version":
      yield* Console.log(getVersion());
      return 0;

    case "inspect": {
      if (Option.isNone(args.inspectType)) {
        yield* Console.log("Usage: repo-lint inspect <layout|rule>");
        return 0;
      }

      yield* runInspect({
        type: args.inspectType.value,
        arg: args.inspectArg,
        configPath: args.configPath,
      });
      return 0;
    }

    case "check": {
      const result = yield* runCheck({
        scope: args.scope,
        format: args.format,
        configPath: args.configPath,
        noCache: args.noCache,
        scanOverrides: args.scanOverrides,
      });

      yield* Console.log(format(result, args.format));

      return result.summary.errors > 0 ? 1 : 0;
    }
  }
});

// Run the program and handle exit codes properly through Effect
const main = Effect.gen(function* () {
  const exitCode = yield* program.pipe(
    Effect.catchAll((error) => {
      // Provide user-friendly messages for common errors
      if (typeof error === "object" && error !== null && "_tag" in error) {
        if (error._tag === "ConfigNotFoundError") {
          return Console.error("No config file found. Create .repo-lint.yaml").pipe(Effect.map(() => 1));
        }
      }
      return Console.error(String(error.message)).pipe(Effect.map(() => 1));
    }),
  );

  return exitCode;
});

Effect.runPromiseExit(main).then((exit) => {
  if (Exit.isSuccess(exit)) {
    process.exitCode = exit.value;
  } else {
    const failure = Cause.failureOption(exit.cause);
    if (Option.isSome(failure)) {
      // Use sync console here as we're outside Effect runtime
      const msg = typeof failure.value === "object" && failure.value !== null && "message" in failure.value
        ? String((failure.value as { message: string }).message)
        : String(failure.value);
      process.stderr.write(`Error: ${msg}\n`);
    } else {
      process.stderr.write("Unknown error occurred\n");
    }
    process.exitCode = 1;
  }
});
