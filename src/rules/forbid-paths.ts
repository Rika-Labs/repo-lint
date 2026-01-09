import { Effect } from "effect";
import type { CheckContext } from "./context.js";
import { addViolation } from "./context.js";
import { matchesAny } from "../core/matcher.js";
import { RuleNames } from "../types/index.js";

export const checkForbidPaths = (ctx: CheckContext): Effect.Effect<void> =>
  Effect.gen(function* () {
    const patterns = ctx.config.rules?.forbidPaths ?? [];
    if (patterns.length === 0) return;

    for (const file of ctx.files) {
      if (matchesAny(file.relativePath, patterns)) {
        yield* addViolation(ctx, {
          path: file.relativePath,
          rule: RuleNames.ForbidPaths,
          message: "path matches forbidden pattern",
        });
      }
    }
  });
