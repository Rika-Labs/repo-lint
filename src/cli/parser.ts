import { Option } from "effect";
import type { OutputFormat } from "../output/index.js";
import type { InspectType } from "../commands/inspect.js";

export type Command = "check" | "inspect" | "help" | "version";

export type ScanOverrides = {
  readonly maxDepth: Option.Option<number>;
  readonly maxFiles: Option.Option<number>;
  readonly timeoutMs: Option.Option<number>;
  readonly concurrency: Option.Option<number>;
  readonly followSymlinks: Option.Option<boolean>;
  readonly useGitignore: Option.Option<boolean>;
};

export type ParsedArgs = {
  readonly command: Command;
  readonly scope: Option.Option<string>;
  readonly format: OutputFormat;
  readonly configPath: Option.Option<string>;
  readonly inspectType: Option.Option<InspectType>;
  readonly inspectArg: Option.Option<string>;
  readonly noCache: Option.Option<boolean>;
  readonly scanOverrides: ScanOverrides;
  readonly errors: readonly string[];
};

const parsePositiveInt = (
  value: string | undefined,
  flag: string,
  errors: string[],
): Option.Option<number> => {
  if (value === undefined || value.startsWith("-")) {
    errors.push(`Missing value for ${flag}`);
    return Option.none();
  }
  const num = Number(value);
  if (!Number.isFinite(num) || num <= 0) {
    errors.push(`Invalid value for ${flag}: ${value}`);
    return Option.none();
  }
  return Option.some(Math.floor(num));
};

export const parseArgs = (argv: readonly string[]): ParsedArgs => {
  let command: Command = "check";
  let scope: Option.Option<string> = Option.none();
  let format: OutputFormat = "console";
  let configPath: Option.Option<string> = Option.none();
  let inspectType: Option.Option<InspectType> = Option.none();
  let inspectArg: Option.Option<string> = Option.none();
  let noCache: Option.Option<boolean> = Option.none();

  let maxDepth: Option.Option<number> = Option.none();
  let maxFiles: Option.Option<number> = Option.none();
  let timeoutMs: Option.Option<number> = Option.none();
  let concurrency: Option.Option<number> = Option.none();
  let followSymlinks: Option.Option<boolean> = Option.none();
  let useGitignore: Option.Option<boolean> = Option.none();

  const errors: string[] = [];

  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === undefined) continue;

    if (arg === "check") {
      command = "check";
    } else if (arg === "inspect") {
      command = "inspect";
      const next = argv[i + 1];
      // Only support "layout" and "rule" - removed "path" as unimplemented
      if (next === "layout" || next === "rule") {
        inspectType = Option.some(next);
        i++;
        if (next === "rule") {
          const nextArg = argv[i + 1];
          if (nextArg !== undefined && !nextArg.startsWith("-")) {
            inspectArg = Option.some(nextArg);
            i++;
          }
        }
      }
    } else if (arg === "--scope") {
      const nextArg = argv[i + 1];
      if (nextArg !== undefined && !nextArg.startsWith("-")) {
        scope = Option.some(nextArg);
        i++;
      } else {
        errors.push("Missing value for --scope");
      }
    } else if (arg === "--json") {
      format = "json";
    } else if (arg === "--sarif") {
      format = "sarif";
    } else if (arg === "--config") {
      const nextArg = argv[i + 1];
      if (nextArg !== undefined && !nextArg.startsWith("-")) {
        configPath = Option.some(nextArg);
        i++;
      } else {
        errors.push("Missing value for --config");
      }
    } else if (arg === "--no-cache") {
      noCache = Option.some(true);
    } else if (arg === "--max-depth") {
      maxDepth = parsePositiveInt(argv[i + 1], arg, errors);
      i++;
    } else if (arg === "--max-files") {
      maxFiles = parsePositiveInt(argv[i + 1], arg, errors);
      i++;
    } else if (arg === "--timeout-ms") {
      timeoutMs = parsePositiveInt(argv[i + 1], arg, errors);
      i++;
    } else if (arg === "--concurrency") {
      concurrency = parsePositiveInt(argv[i + 1], arg, errors);
      i++;
    } else if (arg === "--follow-symlinks") {
      followSymlinks = Option.some(true);
    } else if (arg === "--no-gitignore") {
      useGitignore = Option.some(false);
    } else if (arg === "--gitignore") {
      useGitignore = Option.some(true);
    } else if (arg === "--help" || arg === "-h") {
      command = "help";
    } else if (arg === "--version" || arg === "-v") {
      command = "version";
    }
  }

  return {
    command,
    scope,
    format,
    configPath,
    inspectType,
    inspectArg,
    noCache,
    scanOverrides: {
      maxDepth,
      maxFiles,
      timeoutMs,
      concurrency,
      followSymlinks,
      useGitignore,
    },
    errors,
  };
};
