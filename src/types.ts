export type CaseStyle = "kebab" | "snake" | "camel" | "pascal" | "any";
export type Severity = "error" | "warning";
export type Mode = "strict" | "warn";

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

export type DependencyRule = Record<string, string | readonly string[]>;

export type MirrorRule = {
  readonly source: string;
  readonly target: string;
  readonly pattern?: string;
};

export type WhenRule = Record<string, { readonly requires: readonly string[] }>;

export type BoundaryRule = {
  readonly modules: string;
  readonly publicApi?: string;
  readonly forbidDeepImports?: boolean;
};

export type Rules = {
  readonly forbidPaths?: readonly string[];
  readonly forbidNames?: readonly string[];
  readonly ignorePaths?: readonly string[];
  readonly dependencies?: DependencyRule;
  readonly mirror?: readonly MirrorRule[];
  readonly when?: WhenRule;
  readonly boundaries?: BoundaryRule;
};

export type RepoLintConfig = {
  readonly mode?: Mode;
  readonly extends?: string;
  readonly layout?: LayoutNode;
  readonly ignore?: readonly string[];
  readonly useGitignore?: boolean;
  readonly workspaces?: readonly string[];
  readonly rules?: Rules;
  readonly preset?: string;
};

export type Violation = {
  readonly path: string;
  readonly rule: string;
  readonly message: string;
  readonly severity: Severity;
  readonly expected?: string;
  readonly got?: string;
  readonly suggestions?: readonly string[];
  readonly line?: number;
  readonly column?: number;
};

export type CheckResult = {
  readonly violations: readonly Violation[];
  readonly summary: {
    readonly total: number;
    readonly errors: number;
    readonly warnings: number;
    readonly filesChecked: number;
    readonly duration: number;
  };
};

export type FileEntry = {
  readonly path: string;
  readonly relativePath: string;
  readonly isDirectory: boolean;
  readonly depth: number;
};
