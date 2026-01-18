import { Effect } from "effect";
import type { CheckContext } from "./context.js";
import { addViolation, addWarning } from "./context.js";
import { matches, matchesAny, normalizeUnicode } from "../core/matcher.js";
import { validateCase, getCaseName, suggestCase } from "../core/case.js";
import type { MatchRule, FileEntry } from "../types/index.js";
import { RuleNames } from "../types/index.js";

/**
 * Directory node in the pre-built tree structure.
 * All fields are truly immutable - no casting hacks.
 */
interface DirNode {
  readonly path: string;
  readonly name: string;
  readonly children: ReadonlyMap<string, DirNode>;
  readonly files: readonly string[];
}

/**
 * Mutable builder for constructing DirNode.
 * Used only during tree construction, then converted to immutable.
 */
interface DirNodeBuilder {
  path: string;
  name: string;
  children: Map<string, DirNodeBuilder>;
  files: string[];
}

/**
 * Convert mutable builder to immutable node (recursive).
 */
const freezeNode = (builder: DirNodeBuilder): DirNode => ({
  path: builder.path,
  name: builder.name,
  children: new Map(
    Array.from(builder.children.entries()).map(([k, v]) => [k, freezeNode(v)])
  ),
  files: Object.freeze([...builder.files]) as readonly string[],
});

/**
 * Build a directory tree from the flat file list.
 * Uses iterative approach to avoid stack overflow on deep structures.
 * O(n) where n is the number of files.
 */
const buildDirectoryTree = (files: readonly FileEntry[]): DirNode => {
  const root: DirNodeBuilder = {
    path: "",
    name: "",
    children: new Map(),
    files: [],
  };

  const dirMap = new Map<string, DirNodeBuilder>();
  dirMap.set("", root);

  /**
   * Get or create directory node - ITERATIVE to avoid stack overflow.
   */
  const getOrCreateDir = (path: string): DirNodeBuilder => {
    if (path === "") return root;

    const existing = dirMap.get(path);
    if (existing) return existing;

    // Build path segments from root to target (iterative)
    const parts = path.split("/");
    let current = root;
    let currentPath = "";

    for (const part of parts) {
      currentPath = currentPath === "" ? part : `${currentPath}/${part}`;

      let child = current.children.get(part);
      if (!child) {
        child = {
          path: currentPath,
          name: part,
          children: new Map(),
          files: [],
        };
        current.children.set(part, child);
        dirMap.set(currentPath, child);
      }
      current = child;
    }

    return current;
  };

  // Populate tree with files and directories
  for (const file of files) {
    // Normalize unicode for consistent matching
    const normalizedPath = normalizeUnicode(file.relativePath);

    if (file.isDirectory) {
      getOrCreateDir(normalizedPath);
    } else {
      const lastSlash = normalizedPath.lastIndexOf("/");
      const dirPath = lastSlash === -1 ? "" : normalizedPath.slice(0, lastSlash);
      const fileName = lastSlash === -1 ? normalizedPath : normalizedPath.slice(lastSlash + 1);
      const dir = getOrCreateDir(dirPath);
      dir.files.push(fileName);
    }
  }

  // Convert to immutable structure
  return freezeNode(root);
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

  // Use iterative BFS to avoid stack overflow
  const queue: DirNode[] = [root];

  while (queue.length > 0) {
    const node = queue.shift()!;

    // Check if this directory matches (skip root)
    if (node.path !== "" && matches(node.path, pattern)) {
      // Check exclusions
      if (!exclude || !matchesAny(node.path, exclude)) {
        results.push(node);
      }
    }

    // Add children to queue
    for (const child of node.children.values()) {
      queue.push(child);
    }
  }

  return results;
};

/**
 * Get all direct children (files and subdirectories) of a directory node.
 */
const getDirectChildren = (node: DirNode): readonly string[] => {
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
 * Track violations to prevent duplicates.
 */
const createViolationTracker = () => {
  const seen = new Set<string>();
  return {
    isDuplicate: (path: string, rule: string, message: string): boolean => {
      const key = `${path}|${rule}|${message}`;
      if (seen.has(key)) return true;
      seen.add(key);
      return false;
    },
  };
};

/**
 * Check a single match rule against a matched directory.
 */
const checkMatchRule = (
  ctx: CheckContext,
  rule: MatchRule,
  node: DirNode,
  tracker: ReturnType<typeof createViolationTracker>,
): Effect.Effect<void> =>
  Effect.gen(function* () {
    const allEntries = getDirectChildren(node);

    // Check naming convention on the matched directory itself
    if (rule.case && node.name !== "") {
      if (!validateCase(node.name, rule.case)) {
        const message = `directory name must be ${getCaseName(rule.case)}`;
        if (!tracker.isDuplicate(node.path, RuleNames.Match, message)) {
          yield* addViolation(ctx, {
            path: node.path,
            rule: RuleNames.Match,
            message,
            expected: getCaseName(rule.case),
            got: node.name,
            suggestions: [suggestCase(node.name, rule.case)],
          });
        }
      }
    }

    // Check naming convention on children if childCase is specified
    if (rule.childCase) {
      for (const entry of allEntries) {
        // entry is already a basename, no need to call getBasename
        if (!validateCase(entry, rule.childCase)) {
          const path = joinPath(node.path, entry);
          const message = `name must be ${getCaseName(rule.childCase)}`;
          if (!tracker.isDuplicate(path, RuleNames.Match, message)) {
            yield* addViolation(ctx, {
              path,
              rule: RuleNames.Match,
              message,
              expected: getCaseName(rule.childCase),
              got: entry,
              suggestions: [suggestCase(entry, rule.childCase)],
            });
          }
        }
      }
    }

    // Check required entries
    if (rule.require) {
      for (const required of rule.require) {
        const found = allEntries.some((entry) => matches(entry, required));
        if (!found) {
          const message = `missing required entry: ${required}`;
          if (!tracker.isDuplicate(node.path, RuleNames.Match, message)) {
            yield* addViolation(ctx, {
              path: node.path,
              rule: RuleNames.Match,
              message,
            });
          }
        }
      }
    }

    // Check forbidden entries
    if (rule.forbid) {
      for (const entry of allEntries) {
        if (matchesAny(entry, rule.forbid)) {
          const path = joinPath(node.path, entry);
          const message = "forbidden entry in matched directory";
          if (!tracker.isDuplicate(path, RuleNames.Match, message)) {
            yield* addViolation(ctx, {
              path,
              rule: RuleNames.Match,
              message,
            });
          }
        }
      }
    }

    // Check strict mode (only required + allowed entries permitted)
    if (rule.strict) {
      const allowedPatterns = [...(rule.require ?? []), ...(rule.allow ?? [])];

      // If strict mode with no allowed patterns, reject ALL entries
      if (allowedPatterns.length === 0) {
        for (const entry of allEntries) {
          const path = joinPath(node.path, entry);
          const message = "entry not allowed (strict mode with no allowed patterns)";
          if (!tracker.isDuplicate(path, RuleNames.Match, message)) {
            yield* addViolation(ctx, {
              path,
              rule: RuleNames.Match,
              message,
            });
          }
        }
      } else {
        for (const entry of allEntries) {
          if (!matchesAny(entry, allowedPatterns)) {
            const path = joinPath(node.path, entry);
            const message = "entry not allowed by strict match rule";
            if (!tracker.isDuplicate(path, RuleNames.Match, message)) {
              yield* addViolation(ctx, {
                path,
                rule: RuleNames.Match,
                message,
              });
            }
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

    // Validate patterns before processing
    for (const rule of matchRules) {
      if (!rule.pattern || rule.pattern.trim() === "") {
        yield* addWarning(ctx, {
          path: ".",
          rule: RuleNames.Match,
          message: "match rule has empty pattern - skipping",
        });
      }
    }

    // Filter out invalid rules
    const validRules = matchRules.filter((r) => r.pattern && r.pattern.trim() !== "");
    if (validRules.length === 0) return;

    // Build directory tree once - O(n)
    const root = buildDirectoryTree(ctx.files);

    // Track violations to prevent duplicates across rules
    const tracker = createViolationTracker();

    for (const rule of validRules) {
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
        yield* checkMatchRule(ctx, rule, dir, tracker);
      }
    }
  });
