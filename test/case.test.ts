import { describe, expect, test } from "bun:test";
import { validateCase, suggestCase, getCaseName, isHiddenFile } from "../src/core/case.js";

describe("isHiddenFile", () => {
  test("detects hidden files", () => {
    expect(isHiddenFile(".gitignore")).toBe(true);
    expect(isHiddenFile(".env")).toBe(true);
    expect(isHiddenFile(".DS_Store")).toBe(true);
    expect(isHiddenFile(".eslintrc.json")).toBe(true);
    expect(isHiddenFile("..hidden")).toBe(true);
  });

  test("detects non-hidden files", () => {
    expect(isHiddenFile("index.ts")).toBe(false);
    expect(isHiddenFile("my-file.ts")).toBe(false);
    expect(isHiddenFile("README.md")).toBe(false);
  });
});

describe("validateCase", () => {
  test("hidden files are exempt from case validation", () => {
    // All hidden files should pass any case style
    expect(validateCase(".gitignore", "kebab")).toBe(true);
    expect(validateCase(".DS_Store", "kebab")).toBe(true);
    expect(validateCase(".env", "kebab")).toBe(true);
    expect(validateCase(".eslintrc.json", "kebab")).toBe(true);
    expect(validateCase(".gitignore", "pascal")).toBe(true);
    expect(validateCase(".DS_Store", "snake")).toBe(true);
  });

  test("validates kebab-case", () => {
    expect(validateCase("my-component", "kebab")).toBe(true);
    expect(validateCase("my-component.ts", "kebab")).toBe(true);
    expect(validateCase("MyComponent", "kebab")).toBe(false);
    expect(validateCase("my_component", "kebab")).toBe(false);
  });

  test("validates snake_case", () => {
    expect(validateCase("my_component", "snake")).toBe(true);
    expect(validateCase("my_component.ts", "snake")).toBe(true);
    expect(validateCase("my-component", "snake")).toBe(false);
  });

  test("validates camelCase", () => {
    expect(validateCase("myComponent", "camel")).toBe(true);
    expect(validateCase("myComponent.ts", "camel")).toBe(true);
    expect(validateCase("MyComponent", "camel")).toBe(false);
  });

  test("validates PascalCase", () => {
    expect(validateCase("MyComponent", "pascal")).toBe(true);
    expect(validateCase("MyComponent.tsx", "pascal")).toBe(true);
    expect(validateCase("myComponent", "pascal")).toBe(false);
  });

  test("any case always passes", () => {
    expect(validateCase("anything", "any")).toBe(true);
    expect(validateCase("ANY_THING", "any")).toBe(true);
  });

  test("handles multiple extensions", () => {
    expect(validateCase("my-component.test.ts", "kebab")).toBe(true);
    expect(validateCase("MyComponent.test.tsx", "pascal")).toBe(true);
  });
});

describe("suggestCase", () => {
  test("suggests kebab-case", () => {
    expect(suggestCase("MyComponent.ts", "kebab")).toBe("my-component.ts");
  });

  test("suggests snake_case", () => {
    expect(suggestCase("MyComponent.ts", "snake")).toBe("my_component.ts");
  });

  test("suggests camelCase", () => {
    expect(suggestCase("my-component.ts", "camel")).toBe("myComponent.ts");
  });

  test("suggests PascalCase", () => {
    expect(suggestCase("my-component.ts", "pascal")).toBe("MyComponent.ts");
  });
});

describe("getCaseName", () => {
  test("returns readable names", () => {
    expect(getCaseName("kebab")).toBe("kebab-case");
    expect(getCaseName("snake")).toBe("snake_case");
    expect(getCaseName("camel")).toBe("camelCase");
    expect(getCaseName("pascal")).toBe("PascalCase");
  });
});
