import { Effect } from "effect";
import type { CaseStyle } from "./types.js";

const KEBAB = /^[a-z][a-z0-9]*(-[a-z0-9]+)*$/;
const SNAKE = /^[a-z][a-z0-9]*(_[a-z0-9]+)*$/;
const CAMEL = /^[a-z][a-zA-Z0-9]*$/;
const PASCAL = /^[A-Z][a-zA-Z0-9]*$/;

const extractBaseName = (name: string): string => {
  let base = name;
  while (base.includes(".")) {
    base = base.slice(0, base.lastIndexOf("."));
  }
  return base;
};

export const validateCase = (name: string, style: CaseStyle): boolean => {
  const base = extractBaseName(name);
  switch (style) {
    case "kebab": return KEBAB.test(base);
    case "snake": return SNAKE.test(base);
    case "camel": return CAMEL.test(base);
    case "pascal": return PASCAL.test(base);
    case "any": return true;
  }
};

export const validateCaseEffect = (
  name: string,
  style: CaseStyle,
): Effect.Effect<boolean> => Effect.succeed(validateCase(name, style));

export const toKebab = (s: string): string =>
  s.replace(/([a-z])([A-Z])/g, "$1-$2").replace(/[_\s]+/g, "-").toLowerCase();

export const toSnake = (s: string): string =>
  s.replace(/([a-z])([A-Z])/g, "$1_$2").replace(/[-\s]+/g, "_").toLowerCase();

export const toCamel = (s: string): string => {
  const words = s.replace(/[-_\s]+/g, " ").split(" ");
  return words
    .map((w, i) => (i === 0 ? w.toLowerCase() : w.charAt(0).toUpperCase() + w.slice(1).toLowerCase()))
    .join("");
};

export const toPascal = (s: string): string => {
  const words = s.replace(/[-_\s]+/g, " ").split(" ");
  return words.map((w) => w.charAt(0).toUpperCase() + w.slice(1).toLowerCase()).join("");
};

export const suggestCase = (name: string, style: CaseStyle): string => {
  const base = extractBaseName(name);
  const ext = name.slice(base.length);
  switch (style) {
    case "kebab": return toKebab(base) + ext;
    case "snake": return toSnake(base) + ext;
    case "camel": return toCamel(base) + ext;
    case "pascal": return toPascal(base) + ext;
    default: return name;
  }
};

export const getCaseName = (style: CaseStyle): string => {
  switch (style) {
    case "kebab": return "kebab-case";
    case "snake": return "snake_case";
    case "camel": return "camelCase";
    case "pascal": return "PascalCase";
    default: return style;
  }
};


