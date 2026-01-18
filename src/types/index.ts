import { Schema } from "@effect/schema";

// ============================================================================
// Case Styles
// ============================================================================

export const CaseStyle = Schema.Literal("kebab", "snake", "camel", "pascal", "any");
export type CaseStyle = typeof CaseStyle.Type;

export const Severity = Schema.Literal("error", "warning");
export type Severity = typeof Severity.Type;

export const Mode = Schema.Literal("strict", "warn");
export type Mode = typeof Mode.Type;

// ============================================================================
// Layout Node Type (manual definition for recursive structure)
// ============================================================================

export type LayoutNode = {
  readonly type?: "file" | "dir" | "param" | "many" | "recursive" | "either";
  readonly pattern?: string;
  readonly case?: CaseStyle;
  readonly optional?: boolean;
  readonly required?: boolean;
  readonly strict?: boolean;
  readonly maxDepth?: number;
  readonly maxFiles?: number;
  readonly max?: number;
  readonly min?: number;
  readonly children?: Record<string, LayoutNode>;
  readonly child?: LayoutNode;
  readonly variants?: readonly LayoutNode[];
};

/**
 * Validate if an unknown value is a valid LayoutNode
 */
const isValidLayoutNode = (input: unknown): input is LayoutNode => {
  if (typeof input !== "object" || input === null) {
    return false;
  }
  const obj = input as Record<string, unknown>;
  const allowedKeys = new Set([
    "type",
    "pattern",
    "case",
    "optional",
    "required",
    "strict",
    "maxDepth",
    "maxFiles",
    "max",
    "min",
    "children",
    "child",
    "variants",
  ]);
  for (const key of Object.keys(obj)) {
    if (!allowedKeys.has(key)) {
      return false;
    }
  }

  // Validate type if present
  if (obj["type"] !== undefined) {
    const validTypes = ["file", "dir", "param", "many", "recursive", "either"];
    if (!validTypes.includes(obj["type"] as string)) {
      return false;
    }
  }

  // Validate case if present
  if (obj["case"] !== undefined) {
    const validCases = ["kebab", "snake", "camel", "pascal", "any"];
    if (!validCases.includes(obj["case"] as string)) {
      return false;
    }
  }

  // Validate boolean fields
  const boolFields = ["optional", "required", "strict"];
  for (const field of boolFields) {
    if (obj[field] !== undefined && typeof obj[field] !== "boolean") {
      return false;
    }
  }

  // Validate number fields
  const numFields = ["maxDepth", "maxFiles", "max", "min"];
  for (const field of numFields) {
    if (obj[field] !== undefined && typeof obj[field] !== "number") {
      return false;
    }
  }

  // Validate pattern and string fields
  if (obj["pattern"] !== undefined && typeof obj["pattern"] !== "string") {
    return false;
  }

  // Recursively validate children
  if (obj["children"] !== undefined) {
    if (typeof obj["children"] !== "object" || obj["children"] === null) {
      return false;
    }
    for (const value of Object.values(obj["children"] as Record<string, unknown>)) {
      if (!isValidLayoutNode(value)) {
        return false;
      }
    }
  }

  // Recursively validate child
  if (obj["child"] !== undefined && !isValidLayoutNode(obj["child"])) {
    return false;
  }

  // Recursively validate variants
  if (obj["variants"] !== undefined) {
    if (!Array.isArray(obj["variants"])) {
      return false;
    }
    for (const variant of obj["variants"]) {
      if (!isValidLayoutNode(variant)) {
        return false;
      }
    }
  }

  return true;
};

// Use Schema.declare for proper recursive schema
export const LayoutNodeSchema: Schema.Schema<LayoutNode, LayoutNode> = Schema.declare(
  isValidLayoutNode,
);

// ============================================================================
// Rules Schema
// ============================================================================

export const DependencyRule = Schema.Record({
  key: Schema.String,
  value: Schema.Union(Schema.String, Schema.Array(Schema.String)),
});
export type DependencyRule = typeof DependencyRule.Type;

export const MirrorRule = Schema.Struct({
  source: Schema.String,
  target: Schema.String,
  pattern: Schema.optional(Schema.String),
});
export type MirrorRule = typeof MirrorRule.Type;

export const WhenCondition = Schema.Struct({
  requires: Schema.Array(Schema.String),
});

export const WhenRule = Schema.Record({
  key: Schema.String,
  value: WhenCondition,
});
export type WhenRule = typeof WhenRule.Type;

export const BoundaryRule = Schema.Struct({
  modules: Schema.String,
  publicApi: Schema.optional(Schema.String),
  forbidDeepImports: Schema.optional(Schema.Boolean),
});
export type BoundaryRule = typeof BoundaryRule.Type;

/**
 * Match-based rules allow targeting specific directory patterns
 * and enforcing structure requirements without defining the entire layout tree.
 *
 * @example
 * ```yaml
 * rules:
 *   match:
 *     - pattern: "apps/*\/api/src/modules/*"
 *       require: [controller.ts, service.ts, repo.ts]
 *       allow: [errors.ts, lib]
 *       strict: true
 *       case: kebab
 * ```
 */
export const MatchRule = Schema.Struct({
  /** Glob pattern to match directories */
  pattern: Schema.String,
  /** Patterns to exclude from matching */
  exclude: Schema.optional(Schema.Array(Schema.String)),
  /** Required files/directories that must exist */
  require: Schema.optional(Schema.Array(Schema.String)),
  /** Allowed files/directories (used with strict mode) */
  allow: Schema.optional(Schema.Array(Schema.String)),
  /** Forbidden files/directories */
  forbid: Schema.optional(Schema.Array(Schema.String)),
  /** If true, only required + allowed entries are permitted */
  strict: Schema.optional(Schema.Boolean),
  /** Enforce naming convention for entries */
  case: Schema.optional(CaseStyle),
});
export type MatchRule = typeof MatchRule.Type;

export const Rules = Schema.Struct({
  forbidPaths: Schema.optional(Schema.Array(Schema.String)),
  forbidNames: Schema.optional(Schema.Array(Schema.String)),
  ignorePaths: Schema.optional(Schema.Array(Schema.String)),
  dependencies: Schema.optional(DependencyRule),
  mirror: Schema.optional(Schema.Array(MirrorRule)),
  when: Schema.optional(WhenRule),
  boundaries: Schema.optional(BoundaryRule),
  match: Schema.optional(Schema.Array(MatchRule)),
});
export type Rules = typeof Rules.Type;

// ============================================================================
// Scan Settings Schema
// ============================================================================

export const ScanSettings = Schema.Struct({
  maxDepth: Schema.optional(Schema.Number),
  maxFiles: Schema.optional(Schema.Number),
  followSymlinks: Schema.optional(Schema.Boolean),
  timeoutMs: Schema.optional(Schema.Number),
  concurrency: Schema.optional(Schema.Number),
});
export type ScanSettings = typeof ScanSettings.Type;

// ============================================================================
// Main Config Schema
// ============================================================================

export const RepoLintConfigSchema = Schema.Struct({
  mode: Schema.optional(Mode),
  extends: Schema.optional(Schema.String),
  layout: Schema.optional(LayoutNodeSchema),
  ignore: Schema.optional(Schema.Array(Schema.String)),
  useGitignore: Schema.optional(Schema.Boolean),
  workspaces: Schema.optional(Schema.Array(Schema.String)),
  rules: Schema.optional(Rules),
  scan: Schema.optional(ScanSettings),
  preset: Schema.optional(Schema.String),
});
export type RepoLintConfig = typeof RepoLintConfigSchema.Type;

// ============================================================================
// Violation & Results
// ============================================================================

export const Violation = Schema.Struct({
  path: Schema.String,
  rule: Schema.String,
  message: Schema.String,
  severity: Severity,
  expected: Schema.optional(Schema.String),
  got: Schema.optional(Schema.String),
  suggestions: Schema.optional(Schema.Array(Schema.String)),
  line: Schema.optional(Schema.Number),
  column: Schema.optional(Schema.Number),
});
export type Violation = typeof Violation.Type;

export const CheckSummary = Schema.Struct({
  total: Schema.Number,
  errors: Schema.Number,
  warnings: Schema.Number,
  filesChecked: Schema.Number,
  duration: Schema.Number,
});
export type CheckSummary = typeof CheckSummary.Type;

export const CheckResult = Schema.Struct({
  violations: Schema.Array(Violation),
  summary: CheckSummary,
});
export type CheckResult = typeof CheckResult.Type;

// ============================================================================
// File Entry
// ============================================================================

export type FileEntry = {
  readonly path: string;
  readonly relativePath: string;
  readonly isDirectory: boolean;
  readonly isSymlink: boolean;
  readonly depth: number;
  readonly mtimeMs?: number;
  readonly size?: number;
};

// ============================================================================
// Rule Names (constants to avoid magic strings)
// ============================================================================

export const RuleNames = {
  ForbidPaths: "forbidPaths",
  ForbidNames: "forbidNames",
  Dependencies: "dependencies",
  Mirror: "mirror",
  When: "when",
  Layout: "layout",
  Naming: "naming",
} as const;

export type RuleName = (typeof RuleNames)[keyof typeof RuleNames];
