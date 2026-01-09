import { Effect, Ref } from "effect";
import type { RepoLintConfig, FileEntry, CheckResult } from "../types/index.js";
import { matchesAny } from "../core/matcher.js";
import { createContext } from "./context.js";
import { checkForbidPaths } from "./forbid-paths.js";
import { checkForbidNames } from "./forbid-names.js";
import { checkDependencies } from "./dependencies.js";
import { checkMirror } from "./mirror.js";
import { checkWhen } from "./when.js";
import { checkLayout } from "./layout.js";

export type { CheckContext } from "./context.js";

export const check = (
  config: RepoLintConfig,
  files: readonly FileEntry[],
): Effect.Effect<CheckResult> =>
  Effect.gen(function* () {
    const start = performance.now();

    const ignorePaths = config.rules?.ignorePaths ?? [];
    const filteredFiles = files.filter((f) => !matchesAny(f.relativePath, ignorePaths));

    const ctx = yield* createContext(config, filteredFiles);

    // Run all checks
    yield* Effect.all(
      [
        checkForbidPaths(ctx),
        checkForbidNames(ctx),
        checkDependencies(ctx),
        checkMirror(ctx),
        checkWhen(ctx),
      ],
      { discard: true },
    );

    // Layout check must run after other checks due to matched tracking
    yield* checkLayout(ctx);

    const violations = yield* Ref.get(ctx.violations);
    const duration = performance.now() - start;

    return {
      violations: [...violations],
      summary: {
        total: violations.length,
        errors: violations.filter((v) => v.severity === "error").length,
        warnings: violations.filter((v) => v.severity === "warning").length,
        filesChecked: files.length,
        // Keep 2 decimal places for sub-millisecond precision
        duration: Math.round(duration * 100) / 100,
      },
    };
  });
