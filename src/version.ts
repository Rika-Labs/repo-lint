import { Effect } from "effect";
import { join } from "node:path";
import { readFileSync } from "node:fs";

let cachedVersion: string | null = null;

/**
 * Get version synchronously (uses cached value after first call)
 */
export const getVersion = (): string => {
  if (cachedVersion !== null) {
    return cachedVersion;
  }

  try {
    const packageJsonPath = join(import.meta.dir, "..", "package.json");
    const content = readFileSync(packageJsonPath, "utf-8");
    const parsed = JSON.parse(content) as { version?: string };
    cachedVersion = parsed.version ?? "0.0.0";
    return cachedVersion;
  } catch {
    cachedVersion = "0.0.0";
    return cachedVersion;
  }
};

/**
 * Get version asynchronously using Effect
 */
export const getVersionEffect = (): Effect.Effect<string, never, never> =>
  Effect.sync(() => getVersion());
