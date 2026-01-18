import { Effect } from "effect";
import type { CheckContext } from "./context.js";
import { addViolation, addWarning } from "./context.js";
import { matches, matchesAny, getBasename } from "../core/matcher.js";
import { validateCase, getCaseName } from "../core/case.js";
import type { MatchRule, FileEntry } from "../types/index.js";
import { RuleNames } from "../types/index.js";

/**
 * Directory node in the pre-built tree structure.
 * Built once, traversed efficiently.
 */
interface DirNode {
  readonly path: string;
  readonly name: string;
  readonly children: Map<string, DirNode>;
  readonly files: string[];
}

/**
 * Build a directory tree from the flat file list.
 * O(n) where n is the number of files - single pass.
 */
const buildDirectoryTree = (files: readonly FileEntry[]): DirNode => {
  const root: DirNode = {
    path: "",
    name: "",
    children: new Map(),
    files: [],
  };

  // First pass: create all directory nodes
  const dirMap = new Map<string, DirNode>();
  dirMap.set("", root);

  const getOrCreateDir = (path: string): DirNode => {
    if (path === "") return root;

    const existing = dirMap.get(path);
    if (existing) return existing;

    const parts = path.split("/");
    const name = parts[parts.length - 1] ?? "";
    const parentPath = parts.slice(0, -1).join("/");
    const parent = getOrCreateDir(parentPath);

    const node: DirNode = {
      path,
      name,
      children: new Map(),
      files: [],
    };

    parent.children.set(name, node);
    dirMap.set(path, node);
    return node;
  };

  // Second pass: populate files and ensure directories exist
  for (const file of files) {
    if (file.isDirectory) {
      getOrCreateDir(file.relativePath);
    } else {
      const lastSlash = file.relativePath.lastIndexOf("/");
      const dirPath = lastSlash === -1 ? "" : file.relativePath.slice(0, lastSlash);
      const fileName = lastSlash === -1 ? file.relativePath : file.relativePath.slice(lastSlash + 1);
      const dir = getOrCreateDir(dirPath);
      (dir.files as string[]).push(fileName);
    }
  }

  return root;
};

/**
 * Collect all directories that match a pattern.
 * Traverses the tree once per rule.
 */
const collectMatchingDirs = (
  root: DirNode,
  pattern: string,
  exclude: readonly string[] | undefined,
): DirNode[] => {
  const results: DirNode[] = [];

  const traverse = (node: DirNode): void => {
    // Check if this directory matches
    if (node.path !== "" && matches(node.path, pattern)) {
      // Check exclusions
      if (!exclude || !matchesAny(node.path, exclude)) {
        results.push(node);
      }
    }

    // Recurse into children
    for (const child of node.children.values()) {
      traverse(child);
    }
  };

  traverse(root);
  return results;
};

/**
 * Get all direct children (files and subdirectories) of a directory node.
 */
const getDirectChildren = (node: DirNode): string[] => {
  const entries: string[] = [...node.files];
  for (const child of node.children.values()) {
    entries.push(child.name);
  }
  return entries;
};

/**
 * Construct a proper path, handling empty directory paths.
 */
const joinPath = (dirPath: string, entry: string): string => {
  return dirPath === "" ? entry : `${dirPath}/${entry}`;
};

/**
 * Check a single match rule against a matched directory.
 */
const checkMatchRule = (
  ctx: CheckContext,
  rule: MatchRule,
  node: DirNode,
): Effect.Effect<void> =>
  Effect.gen(function* () {
    const allEntries = getDirectChildren(node);

    // Check naming convention on the matched directory itself
    if (rule.case) {
      if (node.name !== "" && !validateCase(node.name, rule.case)) {
        yield* addViolation(ctx, {
          path: node.path,
          rule: RuleNames.Match,
          message: `directory name must be ${getCaseName(rule.case)}`,
          expected: getCaseName(rule.case),
          got: node.name,
        });
      }
    }

    // Check naming convention on children if childCase is specified
    if (rule.childCase) {
      for (const entry of allEntries) {
        const name = getBasename(entry);
        if (!validateCase(name, rule.childCase)) {
          yield* addViolation(ctx, {
            path: joinPath(node.path, entry),
            rule: RuleNames.Match,
            message: `name must be ${getCaseName(rule.childCase)}`,
            expected: getCaseName(rule.childCase),
            got: name,
          });
        }
      }
    }

    // Check required entries
    if (rule.require) {
      for (const required of rule.require) {
        const found = allEntries.some((entry) => matches(entry, required));
        if (!found) {
          yield* addViolation(ctx, {
            path: node.path,
            rule: RuleNames.Match,
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
            path: joinPath(node.path, entry),
            rule: RuleNames.Match,
            message: "forbidden entry in matched directory",
          });
        }
      }
    }

    // Check strict mode (only required + allowed entries permitted)
    if (rule.strict) {
      const allowedPatterns = [...(rule.require ?? []), ...(rule.allow ?? [])];

      // If strict mode with no allowed patterns, reject ALL entries
      if (allowedPatterns.length === 0) {
        for (const entry of allEntries) {
          yield* addViolation(ctx, {
            path: joinPath(node.path, entry),
            rule: RuleNames.Match,
            message: "entry not allowed (strict mode with no allowed patterns)",
          });
        }
      } else {
        for (const entry of allEntries) {
          if (!matchesAny(entry, allowedPatterns)) {
            yield* addViolation(ctx, {
              path: joinPath(node.path, entry),
              rule: RuleNames.Match,
              message: "entry not allowed by strict match rule",
            });
          }
        }
      }
    }
  });

/**
 * Check all match rules against the filesystem.
 *
 * Performance: O(n + r*d) where n=files, r=rules, d=matched dirs
 * - Builds directory tree once: O(n)
 * - For each rule, traverses tree and checks matched dirs: O(tree_size + matched_dirs)
 */
export const checkMatch = (ctx: CheckContext): Effect.Effect<void> =>
  Effect.gen(function* () {
    const matchRules = ctx.config.rules?.match ?? [];
    if (matchRules.length === 0) return;

    // Build directory tree once - O(n)
    const root = buildDirectoryTree(ctx.files);

    for (const rule of matchRules) {
      // Find all directories matching the pattern
      const matchedDirs = collectMatchingDirs(root, rule.pattern, rule.exclude);

      // Warn if pattern matches nothing (likely config error)
      if (matchedDirs.length === 0) {
        yield* addWarning(ctx, {
          path: ".",
          rule: RuleNames.Match,
          message: `match pattern "${rule.pattern}" did not match any directories`,
        });
        continue;
      }

      // Check each matched directory
      for (const dir of matchedDirs) {
        yield* checkMatchRule(ctx, rule, dir);
      }
    }
  });
