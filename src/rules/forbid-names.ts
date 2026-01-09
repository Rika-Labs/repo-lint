import { Effect } from "effect";
import type { CheckContext } from "./context.js";
import { addViolation } from "./context.js";
import { getBasename } from "../core/matcher.js";
import { RuleNames } from "../types/index.js";

export const checkForbidNames = (ctx: CheckContext): Effect.Effect<void> =>
  Effect.gen(function* () {
    const names = ctx.config.rules?.forbidNames ?? [];
    if (names.length === 0) return;

    for (const file of ctx.files) {
      const basename = getBasename(file.relativePath);
      if (names.includes(basename)) {
        yield* addViolation(ctx, {
          path: file.relativePath,
          rule: RuleNames.ForbidNames,
          message: `filename "${basename}" is forbidden`,
        });
      }
    }
  });
