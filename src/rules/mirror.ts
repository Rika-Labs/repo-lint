import { Effect } from "effect";
import type { CheckContext } from "./context.js";
import { addViolation } from "./context.js";
import { matches } from "../core/matcher.js";
import { RuleNames } from "../types/index.js";

const extractPathSegments = (path: string, pattern: string): Map<string, string> | null => {
  const segments = new Map<string, string>();
  const pathParts = path.split("/");
  const patternParts = pattern.split("/");

  if (pathParts.length < patternParts.length) return null;

  let pathIdx = 0;
  for (let i = 0; i < patternParts.length; i++) {
    const patternPart = patternParts[i];
    if (patternPart === undefined) continue;

    if (patternPart === "*") {
      const pathPart = pathParts[pathIdx];
      if (pathPart === undefined) return null;
      segments.set(`$${i}`, pathPart);
      pathIdx++;
    } else if (patternPart === "**") {
      const remaining = patternParts.length - i - 1;
      const consumed = pathParts.length - remaining - pathIdx;
      if (consumed < 1) return null;
      segments.set(`$${i}`, pathParts.slice(pathIdx, pathIdx + consumed).join("/"));
      pathIdx += consumed;
    } else if (patternPart.includes("*")) {
      const pathPart = pathParts[pathIdx];
      if (pathPart === undefined) return null;
      const escaped = patternPart.replace(/[.+?^${}()|[\]\\]/g, "\\$&").replace(/\*/g, "(.*)");
      const regex = new RegExp(`^${escaped}$`);
      const match = pathPart.match(regex);
      if (!match) return null;
      segments.set(`$${i}`, pathPart);
      pathIdx++;
    } else {
      if (pathParts[pathIdx] !== patternPart) return null;
      pathIdx++;
    }
  }

  return segments;
};

const buildTargetPath = (
  sourcePath: string,
  sourcePattern: string,
  targetPattern: string,
  filePattern?: string,
): string | null => {
  const sourceSegments = extractPathSegments(sourcePath, sourcePattern);
  if (!sourceSegments) return null;

  const sourceParts = sourcePattern.split("/");
  const targetParts = targetPattern.split("/");

  const wildcardIndices: number[] = [];
  sourceParts.forEach((part, idx) => {
    if (part === "*" || part === "**" || part.includes("*")) {
      wildcardIndices.push(idx);
    }
  });

  const resultParts: string[] = [];
  let wildcardUsed = 0;

  for (const targetPart of targetParts) {
    if (targetPart === undefined) continue;

    if (targetPart === "*" || targetPart === "**") {
      const sourceIdx = wildcardIndices[wildcardUsed];
      if (sourceIdx !== undefined) {
        const segment = sourceSegments.get(`$${sourceIdx}`);
        if (segment) {
          resultParts.push(segment);
          wildcardUsed++;
          continue;
        }
      }
      resultParts.push(targetPart);
    } else if (targetPart.includes("*")) {
      const sourceIdx = wildcardIndices[wildcardUsed];
      if (sourceIdx !== undefined) {
        const segment = sourceSegments.get(`$${sourceIdx}`);
        if (segment) {
          let newPart = segment;
          if (filePattern) {
            const [from, to] = filePattern.split(" -> ");
            if (from && to) {
              const fromExt = from.replace("*", "");
              const toExt = to.replace("*", "");
              newPart = segment.replace(fromExt, toExt);
            }
          }
          resultParts.push(newPart);
          wildcardUsed++;
          continue;
        }
      }
      resultParts.push(targetPart);
    } else {
      resultParts.push(targetPart);
    }
  }

  return resultParts.join("/");
};

export const checkMirror = (ctx: CheckContext): Effect.Effect<void> =>
  Effect.gen(function* () {
    const mirrors = ctx.config.rules?.mirror ?? [];

    for (const mirror of mirrors) {
      const sourceFiles = ctx.files.filter(
        (f) => !f.isDirectory && matches(f.relativePath, mirror.source),
      );

      for (const source of sourceFiles) {
        const targetPath = buildTargetPath(
          source.relativePath,
          mirror.source,
          mirror.target,
          mirror.pattern,
        );

        if (targetPath && !ctx.fileSet.has(targetPath)) {
          yield* addViolation(ctx, {
            path: source.relativePath,
            rule: RuleNames.Mirror,
            message: `missing mirrored file: ${targetPath}`,
            expected: targetPath,
          });
        }
      }
    }
  });
