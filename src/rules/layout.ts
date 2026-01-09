import { Effect, Ref } from "effect";
import type { CheckContext } from "./context.js";
import { addViolation, markMatched, isMatched } from "./context.js";
import { validateCase, suggestCase, getCaseName } from "../core/case.js";
import { matchesWithBraces, matchesAny, getBasename } from "../core/matcher.js";
import type { LayoutNode } from "../types/index.js";
import { RuleNames } from "../types/index.js";

export const checkLayoutNode = (
  ctx: CheckContext,
  node: LayoutNode,
  currentPath: string,
): Effect.Effect<boolean> =>
  Effect.gen(function* () {
    const type = node.type ?? "file";

    switch (type) {
      case "file":
        return yield* checkFileNode(ctx, node, currentPath);
      case "dir":
        return yield* checkDirNode(ctx, node, currentPath);
      case "param":
        return yield* checkParamNode(ctx, node, currentPath);
      case "many":
        return yield* checkManyNode(ctx, node, currentPath);
      case "recursive":
        return yield* checkRecursiveNode(ctx, node, currentPath, 0);
      case "either":
        return yield* checkEitherNode(ctx, node, currentPath);
      default:
        return yield* checkFileNode(ctx, node, currentPath);
    }
  });

const checkFileNode = (
  ctx: CheckContext,
  node: LayoutNode,
  path: string,
): Effect.Effect<boolean> =>
  Effect.gen(function* () {
    if (ctx.fileSet.has(path)) {
      yield* markMatched(ctx, path);

      if (node.pattern) {
        const basename = getBasename(path);
        if (!matchesWithBraces(basename, node.pattern)) {
          yield* addViolation(ctx, {
            path,
            rule: RuleNames.Layout,
            message: `file does not match pattern "${node.pattern}"`,
          });
        }
      }

      if (node.case) {
        const basename = getBasename(path);
        if (!validateCase(basename, node.case)) {
          yield* addViolation(ctx, {
            path,
            rule: RuleNames.Naming,
            message: `expected ${getCaseName(node.case)}`,
            got: basename,
            suggestions: [suggestCase(basename, node.case)],
          });
        }
      }

      return true;
    }

    if (node.required) {
      yield* addViolation(ctx, {
        path,
        rule: RuleNames.Layout,
        message: "required file is missing",
      });
    }

    return false;
  });

const checkDirNode = (
  ctx: CheckContext,
  node: LayoutNode,
  path: string,
): Effect.Effect<boolean> =>
  Effect.gen(function* () {
    const exists = path === "" || ctx.dirSet.has(path);

    if (!exists) {
      if (node.required) {
        yield* addViolation(ctx, {
          path,
          rule: RuleNames.Layout,
          message: "required directory is missing",
        });
      }
      return false;
    }

    if (path) yield* markMatched(ctx, path);

    const children = node.children ?? {};

    for (const [name, childNode] of Object.entries(children)) {
      if (name.startsWith("$")) {
        yield* checkLayoutNode(ctx, childNode, path);
      } else {
        const childPath = path ? `${path}/${name}` : name;
        yield* checkLayoutNode(ctx, childNode, childPath);
      }
    }

    if (node.strict) {
      const prefix = path ? `${path}/` : "";
      const allowedNames = new Set(Object.keys(children).filter((n) => !n.startsWith("$")));

      for (const file of ctx.files) {
        if (file.relativePath.startsWith(prefix)) {
          const rest = file.relativePath.slice(prefix.length);
          const firstPart = rest.split("/")[0];
          const matched = yield* isMatched(ctx, file.relativePath);
          if (firstPart && !allowedNames.has(firstPart) && !matched) {
            yield* addViolation(ctx, {
              path: file.relativePath,
              rule: RuleNames.Layout,
              message: `"${firstPart}" is not allowed in ${path || "root"}`,
            });
            // Prevent duplicate "unexpected file" violation later
            yield* markMatched(ctx, file.relativePath);
          }
        }
      }
    }

    return true;
  });

const checkParamNode = (
  ctx: CheckContext,
  node: LayoutNode,
  parentPath: string,
): Effect.Effect<boolean> =>
  Effect.gen(function* () {
    const prefix = parentPath ? `${parentPath}/` : "";
    let found = false;

    for (const entry of ctx.files) {
      if (!entry.relativePath.startsWith(prefix)) continue;

      const rest = entry.relativePath.slice(prefix.length);
      const name = rest.split("/")[0];
      if (!name) continue;

      const fullPath = parentPath ? `${parentPath}/${name}` : name;

      const matched = yield* isMatched(ctx, fullPath);
      if (matched) continue;

      if (node.case && !validateCase(name, node.case)) {
        yield* addViolation(ctx, {
          path: fullPath,
          rule: RuleNames.Naming,
          message: `expected ${getCaseName(node.case)}`,
          got: name,
          suggestions: [suggestCase(name, node.case)],
        });
      }

      if (node.pattern && !matchesWithBraces(name, node.pattern)) continue;

      if (node.child) {
        yield* checkLayoutNode(ctx, node.child, fullPath);
      }

      yield* markMatched(ctx, fullPath);
      found = true;
    }

    return found;
  });

const checkManyNode = (
  ctx: CheckContext,
  node: LayoutNode,
  parentPath: string,
): Effect.Effect<boolean> =>
  Effect.gen(function* () {
    const prefix = parentPath ? `${parentPath}/` : "";
    let count = 0;
    const seen = new Set<string>();

    for (const entry of ctx.files) {
      if (!entry.relativePath.startsWith(prefix)) continue;

      const rest = entry.relativePath.slice(prefix.length);
      const name = rest.split("/")[0];
      if (!name) continue;

      if (seen.has(name)) continue;
      seen.add(name);

      const fullPath = parentPath ? `${parentPath}/${name}` : name;
      const matched = yield* isMatched(ctx, fullPath);
      if (matched) continue;

      if (node.pattern && !matchesWithBraces(name, node.pattern)) continue;

      if (node.case && !validateCase(name, node.case)) {
        yield* addViolation(ctx, {
          path: fullPath,
          rule: RuleNames.Naming,
          message: `expected ${getCaseName(node.case)}`,
          got: name,
          suggestions: [suggestCase(name, node.case)],
        });
      }

      if (node.child) {
        yield* checkLayoutNode(ctx, node.child, fullPath);
      } else {
        yield* markMatched(ctx, fullPath);
      }

      count++;
    }

    if (node.max !== undefined && count > node.max) {
      yield* addViolation(ctx, {
        path: parentPath,
        rule: RuleNames.Layout,
        message: `exceeded maximum count: ${count} > ${node.max}`,
      });
    }

    if (node.min !== undefined && count < node.min) {
      yield* addViolation(ctx, {
        path: parentPath,
        rule: RuleNames.Layout,
        message: `below minimum count: ${count} < ${node.min}`,
      });
    }

    return count > 0;
  });

const checkRecursiveNode = (
  ctx: CheckContext,
  node: LayoutNode,
  parentPath: string,
  depth: number,
): Effect.Effect<boolean> =>
  Effect.gen(function* () {
    if (node.maxDepth !== undefined && depth > node.maxDepth) return false;

    // Mark parent directory as matched to avoid strict-mode duplicates
    if (parentPath && ctx.dirSet.has(parentPath)) {
      yield* markMatched(ctx, parentPath);
    }

    const prefix = parentPath ? `${parentPath}/` : "";
    let found = false;
    const seen = new Set<string>();

    for (const entry of ctx.files) {
      if (!entry.relativePath.startsWith(prefix)) continue;

      const rest = entry.relativePath.slice(prefix.length);
      const name = rest.split("/")[0];
      if (!name) continue;

      if (seen.has(name)) continue;
      seen.add(name);

      const fullPath = parentPath ? `${parentPath}/${name}` : name;
      const matched = yield* isMatched(ctx, fullPath);
      if (matched) continue;

      if (node.case && !validateCase(name, node.case)) {
        yield* addViolation(ctx, {
          path: fullPath,
          rule: RuleNames.Naming,
          message: `expected ${getCaseName(node.case)}`,
          got: name,
          suggestions: [suggestCase(name, node.case)],
        });
      }

      if (node.child) {
        yield* checkLayoutNode(ctx, node.child, fullPath);
      }

      yield* markMatched(ctx, fullPath);
      found = true;

      if (ctx.dirSet.has(fullPath)) {
        yield* checkRecursiveNode(ctx, node, fullPath, depth + 1);
      }
    }

    return found;
  });

const checkEitherNode = (
  ctx: CheckContext,
  node: LayoutNode,
  path: string,
): Effect.Effect<boolean> =>
  Effect.gen(function* () {
    const variants = node.variants ?? [];

    for (const variant of variants) {
      const beforeViolations = yield* Ref.get(ctx.violations);
      const beforeMatched = yield* Ref.get(ctx.matched);
      const matched = yield* checkLayoutNode(ctx, variant, path);
      if (matched) return true;

      // Revert side effects from failed variant to avoid duplicate noise
      yield* Ref.set(ctx.violations, beforeViolations);
      yield* Ref.set(ctx.matched, beforeMatched);
    }

    if (!node.optional) {
      yield* addViolation(ctx, {
        path,
        rule: RuleNames.Layout,
        message: "none of the expected variants matched",
      });
    }

    return false;
  });

export const checkLayout = (ctx: CheckContext): Effect.Effect<void> =>
  Effect.gen(function* () {
    const layout = ctx.config.layout;
    if (!layout) return;

    yield* checkLayoutNode(ctx, layout, "");

    if (ctx.config.mode === "strict") {
      const ignorePaths = ctx.config.rules?.ignorePaths ?? [];
      const matchedSet = yield* Ref.get(ctx.matched);

      for (const file of ctx.files) {
        if (!matchedSet.has(file.relativePath) && !matchesAny(file.relativePath, ignorePaths)) {
          yield* addViolation(ctx, {
            path: file.relativePath,
            rule: RuleNames.Layout,
            message: "unexpected file (not defined in layout)",
          });
        }
      }
    }
  });
