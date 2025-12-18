export type CaseStyle = "kebab" | "snake" | "camel" | "pascal" | "any";

export interface MirrorConfig {
  source: string;
  target: string;
  pattern: string;
}

export interface WhenRequirement {
  requires: string[];
}

export interface DefineConfigOptions {
  /** Extend another config file (supports relative paths and @/ root alias) */
  extends?: string;
  mode?: "strict" | "warn";
  layout: LayoutNode;
  rules?: RulesConfig;
  boundaries?: BoundariesConfig;
  deps?: DepsConfig;
  ignore?: string[];
  useGitignore?: boolean;
  /** Glob patterns to discover workspace configs (e.g., ["apps/*", "packages/*"]) */
  workspaces?: string[];
  /** File dependency rules: source glob -> target glob */
  dependencies?: Record<string, string>;
  /** Mirror structure rules */
  mirror?: MirrorConfig[];
  /** Conditional requirements: if file exists, require others */
  when?: Record<string, WhenRequirement>;
}

export type LayoutNode =
  | DirNode
  | FileNode
  | ParamNode
  | ManyNode
  | RecursiveNode
  | EitherNode;

export interface DirOptions {
  /** Reject files not matching any defined pattern */
  strict?: boolean;
  /** Maximum allowed nesting depth */
  maxDepth?: number;
}

export interface DirNode {
  type: "dir";
  children: Record<string, LayoutNode>;
  optional?: boolean;
  required?: boolean;
  strict?: boolean;
  maxDepth?: number;
}

export interface FileOptions {
  pattern?: string;
  case?: CaseStyle;
}

export interface FileNode {
  type: "file";
  pattern?: string;
  optional?: boolean;
  required?: boolean;
  case?: CaseStyle;
}

export interface ParamNode {
  type: "param";
  name: string;
  case: CaseStyle;
  child: LayoutNode;
}

export interface ManyNode {
  type: "many";
  case?: CaseStyle;
  child: LayoutNode;
  max?: number;
}

export interface RecursiveNode {
  type: "recursive";
  maxDepth?: number;
  child: LayoutNode;
}

export interface EitherNode {
  type: "either";
  variants: LayoutNode[];
}

export interface RulesConfig {
  /** Glob patterns for forbidden paths (supports ! negation) */
  forbidPaths?: string[];
  forbidNames?: string[];
  ignorePaths?: string[];
}

export interface BoundariesConfig {
  modules: string;
  publicApi: string;
  forbidDeepImports?: boolean;
}

export interface DepsAllowRule {
  from: string;
  to: string[];
}

export interface DepsConfig {
  allow?: DepsAllowRule[];
}

export function defineConfig(options: DefineConfigOptions): DefineConfigOptions;

/** Create a directory node */
export function directory(
  children?: Record<string, LayoutNode>,
  options?: DirOptions
): DirNode;
/** @deprecated Use `directory` instead */
export function dir(
  children?: Record<string, LayoutNode>,
  options?: DirOptions
): DirNode;

/** Create a file node */
export function file(pattern?: string): FileNode;
export function file(options: FileOptions): FileNode;

/** Mark a node as optional (may or may not exist) */
export function optional<T extends LayoutNode>(node: T): T & { optional: true };
/** @deprecated Use `optional` instead */
export function opt<T extends LayoutNode>(node: T): T & { optional: true };

/** Mark a node as required (must exist, error if missing) */
export function required<T extends LayoutNode>(node: T): T & { required: true };

export function param(
  options: { case: CaseStyle; name?: string },
  child: LayoutNode
): ParamNode;

export function many(
  options: { case?: CaseStyle; max?: number } | LayoutNode,
  child?: LayoutNode
): ManyNode;

export function recursive(
  options: { maxDepth?: number } | LayoutNode,
  child?: LayoutNode
): RecursiveNode;

export function either(...variants: LayoutNode[]): EitherNode;

export interface NextjsAppRouterOptions {
  routeCase?: CaseStyle;
  maxDepth?: number;
}

export function nextjsAppRouter(options?: NextjsAppRouterOptions): DirNode;
export function nextjsDefaultIgnore(): string[];
export function nextjsDefaultIgnorePaths(): string[];
