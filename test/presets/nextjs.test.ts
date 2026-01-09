import { describe, expect, test } from "bun:test";
import { Effect } from "effect";
import { nextjsPreset } from "../../src/presets/nextjs.js";

describe("nextjsPreset", () => {
  test("returns valid config", async () => {
    const program = Effect.sync(() => nextjsPreset());

    const config = await Effect.runPromise(program);
    expect(config.mode).toBe("strict");
    expect(config.layout).toBeDefined();
    expect(config.ignore).toContain(".next");
    expect(config.ignore).toContain("node_modules");
  });

  test("uses kebab case by default", async () => {
    const program = Effect.sync(() => {
      const config = nextjsPreset();
      const srcLayout = config.layout?.children?.["src"];
      const appLayout = srcLayout?.children?.["app"];
      const routesLayout = appLayout?.children?.["$routes"];
      return routesLayout?.case;
    });

    const routeCase = await Effect.runPromise(program);
    expect(routeCase).toBe("kebab");
  });

  test("allows custom route case", async () => {
    const program = Effect.sync(() => {
      const config = nextjsPreset({ routeCase: "snake" });
      const srcLayout = config.layout?.children?.["src"];
      const appLayout = srcLayout?.children?.["app"];
      const routesLayout = appLayout?.children?.["$routes"];
      return routesLayout?.case;
    });

    const routeCase = await Effect.runPromise(program);
    expect(routeCase).toBe("snake");
  });

  test("includes standard Next.js ignore paths", async () => {
    const program = Effect.sync(() => nextjsPreset());

    const config = await Effect.runPromise(program);
    expect(config.rules?.ignorePaths).toContain("**/.next/**");
    expect(config.rules?.ignorePaths).toContain("**/node_modules/**");
  });

  test("defines app directory structure", async () => {
    const program = Effect.sync(() => {
      const config = nextjsPreset();
      const hasSrcApp = config.layout?.children?.["src"]?.children?.["app"] !== undefined;
      const hasRootApp = config.layout?.children?.["app"] !== undefined;
      return { hasSrcApp, hasRootApp };
    });

    const result = await Effect.runPromise(program);
    expect(result.hasSrcApp).toBe(true);
    expect(result.hasRootApp).toBe(true);
  });

  test("marks optional directories", async () => {
    const program = Effect.sync(() => {
      const config = nextjsPreset();
      return config.layout?.children?.["src"]?.optional;
    });

    const isOptional = await Effect.runPromise(program);
    expect(isOptional).toBe(true);
  });

  test("requires package.json", async () => {
    const program = Effect.sync(() => {
      const config = nextjsPreset();
      return config.layout?.children?.["package.json"]?.optional;
    });

    const isOptional = await Effect.runPromise(program);
    expect(isOptional).toBeUndefined();
  });
});
