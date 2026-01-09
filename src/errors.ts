import { Data } from "effect";

export class ConfigNotFoundError extends Data.TaggedError("ConfigNotFoundError")<{
  readonly path: string;
}> {}

export class ConfigParseError extends Data.TaggedError("ConfigParseError")<{
  readonly path: string;
  readonly cause: unknown;
}> {}

export class ConfigValidationError extends Data.TaggedError("ConfigValidationError")<{
  readonly path: string;
  readonly errors: string[];
}> {}

export class FileSystemError extends Data.TaggedError("FileSystemError")<{
  readonly path: string;
  readonly operation: string;
  readonly cause: unknown;
}> {}

export class ScanError extends Data.TaggedError("ScanError")<{
  readonly root: string;
  readonly cause: unknown;
}> {}

export type RepoLintError =
  | ConfigNotFoundError
  | ConfigParseError
  | ConfigValidationError
  | FileSystemError
  | ScanError;
