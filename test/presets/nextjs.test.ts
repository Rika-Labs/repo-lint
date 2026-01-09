import { describe, expect, test } from "bun:test";
import { nextjsPreset } from "../../src/config/presets/nextjs.js";

describe("nextjsPreset", () => {
  test("returns valid config", () => {
    const config = nextjsPreset();

    expect(config.mode).toBe("strict");
    expect(config.layout).toBeDefined();
    expect(config.ignore).toBeDefined();
  });

  test("uses kebab case by default", () => {
    const config = nextjsPreset();
    const children = config.layout?.children;
    const app = children?.["app"];
    const appChildren = app?.children;
    const route = appChildren?.["$route"];

    // Check that route case defaults to kebab
    expect(route?.case).toBe("kebab");
  });

  test("allows custom route case", () => {
    const config = nextjsPreset({ routeCase: "snake" });
    const children = config.layout?.children;
    const app = children?.["app"];
    const appChildren = app?.children;
    const route = appChildren?.["$route"];

    expect(route?.case).toBe("snake");
  });

  test("includes standard Next.js ignore paths", () => {
    const config = nextjsPreset();

    expect(config.ignore).toContain("node_modules/**");
    expect(config.ignore).toContain(".next/**");
  });

  test("defines app directory structure", () => {
    const config = nextjsPreset();
    const children = config.layout?.children;
    const app = children?.["app"];

    expect(app?.type).toBe("dir");
    expect(app?.children?.["page.tsx"]).toBeDefined();
    expect(app?.children?.["layout.tsx"]).toBeDefined();
  });

  test("marks optional directories", () => {
    const config = nextjsPreset();
    const children = config.layout?.children;

    expect(children?.["components"]?.optional).toBe(true);
    expect(children?.["lib"]?.optional).toBe(true);
    expect(children?.["hooks"]?.optional).toBe(true);
  });

  test("requires package.json", () => {
    const config = nextjsPreset();

    expect(config.layout?.children?.["package.json"]?.required).toBe(true);
  });
});
