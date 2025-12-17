export interface DefineConfigOptions {
  mode?: "strict" | "warn";
  layout: LayoutNode;
  rules?: RulesConfig;
  boundaries?: BoundariesConfig;
  deps?: DepsConfig;
}

export type LayoutNode =
  | DirNode
  | FileNode
  | ParamNode
  | ManyNode;

export interface DirNode {
  type: "dir";
  children: Record<string, LayoutNode>;
  optional?: boolean;
}

export interface FileNode {
  type: "file";
  pattern?: string;
  optional?: boolean;
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
}

export type CaseStyle = "kebab" | "snake" | "camel" | "pascal" | "any";

export interface RulesConfig {
  forbidPaths?: string[];
  forbidNames?: string[];
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

export function dir(children?: Record<string, LayoutNode>): DirNode;
export function file(pattern?: string): FileNode;
export function opt<T extends LayoutNode>(node: T): T & { optional: true };
export function param(
  options: { case: CaseStyle; name?: string },
  child: LayoutNode
): ParamNode;
export function many(
  options: { case?: CaseStyle } | LayoutNode,
  child?: LayoutNode
): ManyNode;
