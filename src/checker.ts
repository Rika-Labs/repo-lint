import { Effect, Ref } from "effect";
import type {
  RepoLintConfig,
  LayoutNode,
  FileEntry,
  Violation,
  CheckResult,
  Severity,
} from "./types.js";
import { validateCase, suggestCase, getCaseName } from "./case.js";
import { matches, matchesAny, matchesWithBraces, getBasename, getParent } from "./matcher.js";

type CheckContext = {
  readonly config: RepoLintConfig;
  readonly files: readonly FileEntry[];
  readonly fileSet: Set<string>;
  readonly dirSet: Set<string>;
  readonly violations: Ref.Ref<Violation[]>;
  readonly matched: Set<string>;
};

const getSeverity = (mode: "strict" | "warn" | undefined): Severity =>
  mode === "strict" ? "error" : "warning";

const addViolation = (
  ctx: CheckContext,
  violation: Omit<Violation, "severity">,
): Effect.Effect<void> =>
  Ref.update(ctx.violations, (vs) => [
    ...vs,
    { ...violation, severity: getSeverity(ctx.config.mode) },
  ]);

const checkForbidPaths = (ctx: CheckContext): Effect.Effect<void> =>
  Effect.gen(function* () {
    const patterns = ctx.config.rules?.forbidPaths ?? [];
    if (patterns.length === 0) return;

    for (const file of ctx.files) {
      if (matchesAny(file.relativePath, patterns)) {
        yield* addViolation(ctx, {
          path: file.relativePath,
          rule: "forbidPaths",
          message: "path matches forbidden pattern",
        });
      }
    }
  });

const checkForbidNames = (ctx: CheckContext): Effect.Effect<void> =>
  Effect.gen(function* () {
    const names = ctx.config.rules?.forbidNames ?? [];
    if (names.length === 0) return;

    for (const file of ctx.files) {
      const basename = getBasename(file.relativePath);
      if (names.includes(basename)) {
        yield* addViolation(ctx, {
          path: file.relativePath,
          rule: "forbidNames",
          message: `filename "${basename}" is forbidden`,
        });
      }
    }
  });

const checkDependencies = (ctx: CheckContext): Effect.Effect<void> =>
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
              rule: "dependencies",
              message: `files matching "${sourcePattern}" require "${targetPattern}" to exist`,
            });
          }
        }
      }
    }
  });

const checkMirror = (ctx: CheckContext): Effect.Effect<void> =>
  Effect.gen(function* () {
    const mirrors = ctx.config.rules?.mirror ?? [];

    for (const mirror of mirrors) {
      const sourceFiles = ctx.files.filter(
        (f) => !f.isDirectory && matches(f.relativePath, mirror.source),
      );

      for (const source of sourceFiles) {
        const sourceDir = getParent(mirror.source).replace(/\*/g, "");
        const targetDir = getParent(mirror.target).replace(/\*/g, "");
        const basename = getBasename(source.relativePath);

        let targetName = basename;
        if (mirror.pattern) {
          const [from, to] = mirror.pattern.split(" -> ");
          if (from && to) {
            const fromExt = from.replace("*", "");
            const toExt = to.replace("*", "");
            targetName = basename.replace(fromExt, toExt);
          }
        }

        const relativeSrcPath = source.relativePath.slice(sourceDir.length);
        const targetPath = targetDir + relativeSrcPath.replace(basename, targetName);

        if (!ctx.fileSet.has(targetPath)) {
          yield* addViolation(ctx, {
            path: source.relativePath,
            rule: "mirror",
            message: `missing mirrored file: ${targetPath}`,
            expected: targetPath,
          });
        }
      }
    }
  });

const checkWhen = (ctx: CheckContext): Effect.Effect<void> =>
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
                rule: "when",
                message: `"${trigger}" requires "${required}" to exist`,
                expected: requiredPath,
              });
            }
          }
        }
      }
    }
  });

const checkLayoutNode = (
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
      ctx.matched.add(path);

      if (node.pattern) {
        const basename = getBasename(path);
        if (!matchesWithBraces(basename, node.pattern)) {
          yield* addViolation(ctx, {
            path,
            rule: "layout",
            message: `file does not match pattern "${node.pattern}"`,
          });
        }
      }

      if (node.case) {
        const basename = getBasename(path);
        if (!validateCase(basename, node.case)) {
          yield* addViolation(ctx, {
            path,
            rule: "naming",
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
        rule: "layout",
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
          rule: "layout",
          message: "required directory is missing",
        });
      }
      return false;
    }

    if (path) ctx.matched.add(path);

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
          if (firstPart && !allowedNames.has(firstPart) && !ctx.matched.has(file.relativePath)) {
            yield* addViolation(ctx, {
              path: file.relativePath,
              rule: "layout",
              message: `"${firstPart}" is not allowed in ${path || "root"}`,
            });
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

      if (ctx.matched.has(fullPath)) continue;

      if (node.case && !validateCase(name, node.case)) {
        yield* addViolation(ctx, {
          path: fullPath,
          rule: "naming",
          message: `expected ${getCaseName(node.case)}`,
          got: name,
          suggestions: [suggestCase(name, node.case)],
        });
      }

      if (node.pattern && !matchesWithBraces(name, node.pattern)) continue;

      if (node.child) {
        yield* checkLayoutNode(ctx, node.child, fullPath);
      }

      ctx.matched.add(fullPath);
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
      if (ctx.matched.has(fullPath)) continue;

      if (node.pattern && !matchesWithBraces(name, node.pattern)) continue;

      if (node.case && !validateCase(name, node.case)) {
        yield* addViolation(ctx, {
          path: fullPath,
          rule: "naming",
          message: `expected ${getCaseName(node.case)}`,
          got: name,
          suggestions: [suggestCase(name, node.case)],
        });
      }

      if (node.child) {
        yield* checkLayoutNode(ctx, node.child, fullPath);
      } else {
        ctx.matched.add(fullPath);
      }

      count++;
    }

    if (node.max !== undefined && count > node.max) {
      yield* addViolation(ctx, {
        path: parentPath,
        rule: "layout",
        message: `exceeded maximum count: ${count} > ${node.max}`,
      });
    }

    if (node.min !== undefined && count < node.min) {
      yield* addViolation(ctx, {
        path: parentPath,
        rule: "layout",
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
      if (ctx.matched.has(fullPath)) continue;

      if (node.case && !validateCase(name, node.case)) {
        yield* addViolation(ctx, {
          path: fullPath,
          rule: "naming",
          message: `expected ${getCaseName(node.case)}`,
          got: name,
          suggestions: [suggestCase(name, node.case)],
        });
      }

      if (node.child) {
        yield* checkLayoutNode(ctx, node.child, fullPath);
      }

      ctx.matched.add(fullPath);
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
      const matched = yield* checkLayoutNode(ctx, variant, path);
      if (matched) return true;
    }

    if (!node.optional) {
      yield* addViolation(ctx, {
        path,
        rule: "layout",
        message: "none of the expected variants matched",
      });
    }

    return false;
  });

const checkLayout = (ctx: CheckContext): Effect.Effect<void> =>
  Effect.gen(function* () {
    const layout = ctx.config.layout;
    if (!layout) return;

    yield* checkLayoutNode(ctx, layout, "");

    if (ctx.config.mode === "strict") {
      const ignorePaths = ctx.config.rules?.ignorePaths ?? [];

      for (const file of ctx.files) {
        if (!ctx.matched.has(file.relativePath) && !matchesAny(file.relativePath, ignorePaths)) {
          yield* addViolation(ctx, {
            path: file.relativePath,
            rule: "layout",
            message: "unexpected file (not defined in layout)",
          });
        }
      }
    }
  });

export const check = (
  config: RepoLintConfig,
  files: readonly FileEntry[],
): Effect.Effect<CheckResult> =>
  Effect.gen(function* () {
    const start = performance.now();

    const ignorePaths = config.rules?.ignorePaths ?? [];
    const filteredFiles = files.filter((f) => !matchesAny(f.relativePath, ignorePaths));

    const violationsRef = yield* Ref.make<Violation[]>([]);

    const ctx: CheckContext = {
      config,
      files: filteredFiles,
      fileSet: new Set(filteredFiles.filter((f) => !f.isDirectory).map((f) => f.relativePath)),
      dirSet: new Set(filteredFiles.filter((f) => f.isDirectory).map((f) => f.relativePath)),
      violations: violationsRef,
      matched: new Set<string>(),
    };

    yield* checkForbidPaths(ctx);
    yield* checkForbidNames(ctx);
    yield* checkDependencies(ctx);
    yield* checkMirror(ctx);
    yield* checkWhen(ctx);
    yield* checkLayout(ctx);

    const violations = yield* Ref.get(violationsRef);
    const duration = performance.now() - start;

    return {
      violations,
      summary: {
        total: violations.length,
        errors: violations.filter((v) => v.severity === "error").length,
        warnings: violations.filter((v) => v.severity === "warning").length,
        filesChecked: files.length,
        duration: Math.round(duration),
      },
    };
  });
