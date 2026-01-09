import { Data } from "effect";

export class ConfigNotFoundError extends Data.TaggedError("ConfigNotFoundError")<{
  readonly path: string;
}> {
  override get message(): string {
    return `Config file not found: ${this.path}`;
  }
}

export class ConfigParseError extends Data.TaggedError("ConfigParseError")<{
  readonly path: string;
  readonly cause: unknown;
}> {
  override get message(): string {
    return `Failed to parse config at ${this.path}: ${String(this.cause)}`;
  }
}

export class ConfigValidationError extends Data.TaggedError("ConfigValidationError")<{
  readonly path: string;
  readonly errors: readonly string[];
}> {
  override get message(): string {
    return `Config validation failed at ${this.path}:\n${this.errors.join("\n")}`;
  }
}

export class CircularExtendsError extends Data.TaggedError("CircularExtendsError")<{
  readonly path: string;
  readonly chain: readonly string[];
}> {
  override get message(): string {
    return `Circular extends detected: ${[...this.chain, this.path].join(" -> ")}`;
  }
}

export class PathTraversalError extends Data.TaggedError("PathTraversalError")<{
  readonly path: string;
  readonly configPath: string;
}> {
  override get message(): string {
    return `Path traversal detected in extends: "${this.path}" from ${this.configPath}`;
  }
}

export class FileSystemError extends Data.TaggedError("FileSystemError")<{
  readonly path: string;
  readonly operation: string;
  readonly cause: unknown;
}> {
  override get message(): string {
    return `Filesystem error during ${this.operation} on ${this.path}: ${String(this.cause)}`;
  }
}

export class ScanError extends Data.TaggedError("ScanError")<{
  readonly root: string;
  readonly cause: unknown;
}> {
  override get message(): string {
    return `Failed to scan directory ${this.root}: ${String(this.cause)}`;
  }
}

export class SymlinkLoopError extends Data.TaggedError("SymlinkLoopError")<{
  readonly path: string;
  readonly target: string;
}> {
  override get message(): string {
    return `Symlink loop detected: ${this.path} -> ${this.target}`;
  }
}

export class MaxDepthExceededError extends Data.TaggedError("MaxDepthExceededError")<{
  readonly path: string;
  readonly depth: number;
  readonly maxDepth: number;
}> {
  override get message(): string {
    return `Max depth exceeded at ${this.path}: ${this.depth} > ${this.maxDepth}`;
  }
}

export class MaxFilesExceededError extends Data.TaggedError("MaxFilesExceededError")<{
  readonly count: number;
  readonly maxFiles: number;
}> {
  override get message(): string {
    return `Max files exceeded: ${this.count} > ${this.maxFiles}`;
  }
}

export type RepoLintError =
  | ConfigNotFoundError
  | ConfigParseError
  | ConfigValidationError
  | CircularExtendsError
  | PathTraversalError
  | FileSystemError
  | ScanError
  | SymlinkLoopError
  | MaxDepthExceededError
  | MaxFilesExceededError;
