import { Effect, Ref } from "effect";
import type { RepoLintConfig, FileEntry, Violation, Severity } from "../types/index.js";

export type CheckContext = {
  readonly config: RepoLintConfig;
  readonly files: readonly FileEntry[];
  readonly fileSet: ReadonlySet<string>;
  readonly dirSet: ReadonlySet<string>;
  readonly violations: Ref.Ref<readonly Violation[]>;
  readonly matched: Ref.Ref<ReadonlySet<string>>;
};

export const getSeverity = (mode: "strict" | "warn" | undefined): Severity =>
  mode === "strict" ? "error" : "warning";

export const addViolation = (
  ctx: CheckContext,
  violation: Omit<Violation, "severity">,
): Effect.Effect<void> =>
  Ref.update(ctx.violations, (vs) => [
    ...vs,
    { ...violation, severity: getSeverity(ctx.config.mode) },
  ]);

export const markMatched = (
  ctx: CheckContext,
  path: string,
): Effect.Effect<void> =>
  Ref.update(ctx.matched, (set) => new Set([...set, path]));

export const isMatched = (
  ctx: CheckContext,
  path: string,
): Effect.Effect<boolean> =>
  Ref.get(ctx.matched).pipe(Effect.map((set) => set.has(path)));

export const createContext = (
  config: RepoLintConfig,
  files: readonly FileEntry[],
): Effect.Effect<CheckContext> =>
  Effect.gen(function* () {
    const violationsRef = yield* Ref.make<readonly Violation[]>([]);
    const matchedRef = yield* Ref.make<ReadonlySet<string>>(new Set());

    return {
      config,
      files,
      fileSet: new Set(files.filter((f) => !f.isDirectory).map((f) => f.relativePath)),
      dirSet: new Set(files.filter((f) => f.isDirectory).map((f) => f.relativePath)),
      violations: violationsRef,
      matched: matchedRef,
    };
  });
