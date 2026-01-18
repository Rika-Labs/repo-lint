import { Effect, Option, Console } from "effect";
import { findConfig, loadConfig } from "../config/index.js";
import { ConfigNotFoundError, ConfigParseError } from "../errors.js";
import type { Rules } from "../types/index.js";

export type InspectType = "layout" | "rule";

export type InspectOptions = {
  readonly type: InspectType;
  readonly arg: Option.Option<string>;
  readonly configPath: Option.Option<string>;
};

export const runInspect = (
  options: InspectOptions,
): Effect.Effect<void, ConfigNotFoundError | ConfigParseError, never> =>
  Effect.gen(function* () {
    const root = process.cwd();

    const configPath = Option.isSome(options.configPath)
      ? Option.some(options.configPath.value)
      : yield* findConfig(root);

    if (Option.isNone(configPath)) {
      yield* Console.error("No config file found");
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

    switch (options.type) {
      case "layout":
        if (config.layout !== undefined) {
          yield* Console.log(JSON.stringify(config.layout, null, 2));
        } else {
          yield* Console.log("No layout defined in config");
        }
        return;

      case "rule":
        if (Option.isNone(options.arg)) {
          yield* Console.error("Usage: repo-lint inspect rule <rule-name>");
          yield* Console.log("\nAvailable rules:");
          yield* Console.log("  forbidPaths   - Patterns of forbidden paths");
          yield* Console.log("  forbidNames   - List of forbidden file names");
          yield* Console.log("  ignorePaths   - Patterns to ignore in strict mode");
          yield* Console.log("  dependencies  - File dependency requirements");
          yield* Console.log("  mirror        - Mirror structure rules");
          yield* Console.log("  when          - Conditional requirements");
          yield* Console.log("  boundaries    - Module boundary rules");
          yield* Console.log("  match         - Pattern-based directory validation rules");
          return;
        }

        const rules = config.rules ?? ({} as Rules);
        const ruleName = options.arg.value;
        const ruleValue = getRuleByName(rules, ruleName);

        if (Option.isSome(ruleValue)) {
          yield* Console.log(JSON.stringify(ruleValue.value, null, 2));
        } else {
          yield* Console.log(`Rule "${ruleName}" not found or not configured`);
        }
        return;
    }
  });

const getRuleByName = (rules: Rules, name: string): Option.Option<unknown> => {
  switch (name) {
    case "forbidPaths":
      return Option.fromNullable(rules.forbidPaths);
    case "forbidNames":
      return Option.fromNullable(rules.forbidNames);
    case "ignorePaths":
      return Option.fromNullable(rules.ignorePaths);
    case "dependencies":
      return Option.fromNullable(rules.dependencies);
    case "mirror":
      return Option.fromNullable(rules.mirror);
    case "when":
      return Option.fromNullable(rules.when);
    case "boundaries":
      return Option.fromNullable(rules.boundaries);
    case "match":
      return Option.fromNullable(rules.match);
    default:
      return Option.none();
  }
};
