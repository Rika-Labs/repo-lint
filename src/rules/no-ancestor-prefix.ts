import { Effect } from "effect";
import type { CheckContext } from "./context.js";
import { addViolation } from "./context.js";
import { matches, matchesAny, normalizePath } from "../core/matcher.js";
import { RuleNames } from "../types/index.js";

const getStem = (basename: string): string => {
  const dotIndex = basename.indexOf(".");
  return dotIndex === -1 ? basename : basename.slice(0, dotIndex);
};

export const checkNoAncestorPrefix = (ctx: CheckContext): Effect.Effect<void> =>
  Effect.gen(function* () {
    const rules = ctx.config.rules?.noAncestorPrefix ?? [];

    for (const rule of rules) {
      const files = ctx.files.filter(
        (file) =>
          !file.isDirectory &&
          matches(file.relativePath, rule.pattern) &&
          !matchesAny(file.relativePath, rule.exclude ?? []),
      );

      for (const file of files) {
        const path = normalizePath(file.relativePath);
        const segments = path.split("/");
        const basename = segments.at(-1) ?? "";
        const stem = getStem(basename);
        const rootIndex = segments.lastIndexOf(rule.root);

        if (rootIndex === -1) {
          yield* addViolation(ctx, {
            path,
            rule: RuleNames.NoAncestorPrefix,
            message: `structural root "${rule.root}" not found in matched path`,
            expected: rule.root,
          });
          continue;
        }

        const repeatedAncestor = segments
          .slice(rootIndex + 1, -1)
          .find((ancestor) => stem === ancestor || stem.startsWith(`${ancestor}-`));

        if (repeatedAncestor !== undefined) {
          yield* addViolation(ctx, {
            path,
            rule: RuleNames.NoAncestorPrefix,
            message: `file name must not repeat ancestor "${repeatedAncestor}"`,
            expected: `name not equal to or prefixed by "${repeatedAncestor}"`,
            got: basename,
          });
        }
      }
    }
  });
