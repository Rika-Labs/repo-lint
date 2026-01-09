import { Effect } from "effect";
import type { CheckContext } from "./context.js";
import { addViolation } from "./context.js";
import { RuleNames } from "../types/index.js";

export const checkWhen = (ctx: CheckContext): Effect.Effect<void> =>
  Effect.gen(function* () {
    const when = ctx.config.rules?.when;
    if (!when) return;

    for (const dir of ctx.dirSet) {
      for (const [trigger, condition] of Object.entries(when)) {
        const triggerPath = dir ? `${dir}/${trigger}` : trigger;

        if (ctx.fileSet.has(triggerPath)) {
          for (const required of condition.requires) {
            const requiredPath = dir ? `${dir}/${required}` : required;
            if (!ctx.fileSet.has(requiredPath)) {
              yield* addViolation(ctx, {
                path: triggerPath,
                rule: RuleNames.When,
                message: `"${trigger}" requires "${required}" to exist`,
                expected: requiredPath,
              });
            }
          }
        }
      }
    }
  });
