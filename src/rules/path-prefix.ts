import { Effect } from "effect";
import type { CheckContext } from "./context.js";
import { addViolation } from "./context.js";
import { matches, normalizePath } from "../core/matcher.js";
import { RuleNames } from "../types/index.js";

export const checkPathPrefix = (ctx: CheckContext): Effect.Effect<void> =>
  Effect.gen(function* () {
    const rules = ctx.config.rules?.pathPrefix ?? [];

    for (const rule of rules) {
      const files = ctx.files.filter(
        (file) => !file.isDirectory && matches(file.relativePath, rule.pattern),
      );

      for (const file of files) {
        const path = normalizePath(file.relativePath);
        const segments = path.split("/");
        const basename = segments.at(-1) ?? "";
        const rootIndex = segments.lastIndexOf(rule.root);

        if (rootIndex === -1) {
          yield* addViolation(ctx, {
            path,
            rule: RuleNames.PathPrefix,
            message: `structural root "${rule.root}" not found in matched path`,
            expected: rule.root,
          });
          continue;
        }

        const prefix = segments.slice(rootIndex + 1, -1).join("-");
        if (prefix !== "" && !basename.startsWith(prefix)) {
          yield* addViolation(ctx, {
            path,
            rule: RuleNames.PathPrefix,
            message: `file name must start with directory prefix "${prefix}"`,
            expected: prefix,
            got: basename,
          });
        }
      }
    }
  });
