import { Effect } from "effect";
import type { CheckContext } from "./context.js";
import { addViolation } from "./context.js";
import { matches } from "../core/matcher.js";
import { RuleNames } from "../types/index.js";

export const checkDependencies = (ctx: CheckContext): Effect.Effect<void> =>
  Effect.gen(function* () {
    const deps = ctx.config.rules?.dependencies;
    if (!deps) return;

    for (const [sourcePattern, targetPatterns] of Object.entries(deps)) {
      const targets = Array.isArray(targetPatterns) ? targetPatterns : [targetPatterns];
      const sourceFiles = ctx.files.filter((f) => matches(f.relativePath, sourcePattern));

      if (sourceFiles.length > 0) {
        for (const targetPattern of targets) {
          const hasTarget = ctx.files.some((f) => matches(f.relativePath, targetPattern));
          if (!hasTarget) {
            yield* addViolation(ctx, {
              path: sourcePattern,
              rule: RuleNames.Dependencies,
              message: `files matching "${sourcePattern}" require "${targetPattern}" to exist`,
            });
          }
        }
      }
    }
  });
