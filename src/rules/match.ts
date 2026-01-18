import { Effect } from "effect";
import type { CheckContext } from "./context.js";
import { addViolation } from "./context.js";
import { matches, matchesAny, getBasename, getParent } from "../core/matcher.js";
import { validateCase, getCaseName } from "../core/case.js";
import type { MatchRule } from "../types/index.js";

const RULE_NAME = "match";

/**
 * Get all unique directory paths from the file list
 */
const getDirectories = (ctx: CheckContext): Set<string> => {
  const dirs = new Set<string>();
  for (const file of ctx.files) {
    if (file.isDirectory) {
      dirs.add(file.relativePath);
    } else {
      // Add parent directories of files
      let parent = getParent(file.relativePath);
      while (parent && parent !== ".") {
        dirs.add(parent);
        parent = getParent(parent);
      }
    }
  }
  return dirs;
};

/**
 * Get direct children of a directory
 */
const getDirectChildren = (
  ctx: CheckContext,
  dirPath: string,
): { files: string[]; dirs: string[] } => {
  const prefix = dirPath === "" ? "" : `${dirPath}/`;
  const files: string[] = [];
  const dirs: string[] = [];

  for (const file of ctx.files) {
    if (!file.relativePath.startsWith(prefix)) continue;

    const rest = file.relativePath.slice(prefix.length);
    // Skip if it's a nested path (contains more slashes)
    if (rest.includes("/")) continue;
    // Skip empty (the directory itself)
    if (rest === "") continue;

    if (file.isDirectory) {
      dirs.push(rest);
    } else {
      files.push(rest);
    }
  }

  return { files, dirs };
};

/**
 * Check a single match rule against a matched directory
 */
const checkMatchRule = (
  ctx: CheckContext,
  rule: MatchRule,
  dirPath: string,
): Effect.Effect<void> =>
  Effect.gen(function* () {
    const children = getDirectChildren(ctx, dirPath);
    const allEntries = [...children.files, ...children.dirs];

    // Check required entries
    if (rule.require) {
      for (const required of rule.require) {
        const found = allEntries.some((entry) => matches(entry, required));
        if (!found) {
          yield* addViolation(ctx, {
            path: dirPath,
            rule: RULE_NAME,
            message: `missing required entry: ${required}`,
          });
        }
      }
    }

    // Check forbidden entries
    if (rule.forbid) {
      for (const entry of allEntries) {
        if (matchesAny(entry, rule.forbid)) {
          yield* addViolation(ctx, {
            path: `${dirPath}/${entry}`,
            rule: RULE_NAME,
            message: "forbidden entry in matched directory",
          });
        }
      }
    }

    // Check strict mode (only required + allowed entries permitted)
    if (rule.strict) {
      const allowedPatterns = [...(rule.require ?? []), ...(rule.allow ?? [])];

      for (const entry of allEntries) {
        const isAllowed = allowedPatterns.length === 0 || matchesAny(entry, allowedPatterns);
        if (!isAllowed) {
          yield* addViolation(ctx, {
            path: `${dirPath}/${entry}`,
            rule: RULE_NAME,
            message: "entry not allowed by strict match rule",
          });
        }
      }
    }

    // Check naming convention
    if (rule.case) {
      for (const entry of allEntries) {
        const name = getBasename(entry);
        if (!validateCase(name, rule.case)) {
          yield* addViolation(ctx, {
            path: `${dirPath}/${entry}`,
            rule: RULE_NAME,
            message: `name must be ${getCaseName(rule.case)}`,
            expected: getCaseName(rule.case),
            got: name,
          });
        }
      }
    }
  });

/**
 * Check all match rules against the filesystem
 */
export const checkMatch = (ctx: CheckContext): Effect.Effect<void> =>
  Effect.gen(function* () {
    const matchRules = ctx.config.rules?.match ?? [];
    if (matchRules.length === 0) return;

    const directories = getDirectories(ctx);

    for (const rule of matchRules) {
      // Find directories that match the pattern
      for (const dirPath of directories) {
        if (!matches(dirPath, rule.pattern)) continue;

        // Check exclusions
        if (rule.exclude && matchesAny(dirPath, rule.exclude)) continue;

        // Run checks for this matched directory
        yield* checkMatchRule(ctx, rule, dirPath);
      }
    }
  });
