import { describe, expect, test } from "bun:test";
import { validateCase, suggestCase, getCaseName } from "../src/core/case.js";

describe("validateCase", () => {
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
