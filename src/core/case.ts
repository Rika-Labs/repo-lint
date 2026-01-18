import type { CaseStyle } from "../types/index.js";

const KEBAB_REGEX = /^[a-z][a-z0-9]*(-[a-z0-9]+)*(\.[a-z0-9]+)*$/;
const SNAKE_REGEX = /^[a-z][a-z0-9]*(_[a-z0-9]+)*(\.[a-z0-9]+)*$/;
const CAMEL_REGEX = /^[a-z][a-zA-Z0-9]*(\.[a-zA-Z0-9]+)*$/;
const PASCAL_REGEX = /^[A-Z][a-zA-Z0-9]*(\.[a-zA-Z0-9]+)*$/;

/**
 * Check if a filename is a hidden file (starts with dot).
 * Hidden files are exempt from case validation as they follow
 * their own conventions (e.g., .gitignore, .env, .eslintrc).
 */
export const isHiddenFile = (name: string): boolean => name.startsWith(".");

export const validateCase = (name: string, style: CaseStyle): boolean => {
  // Hidden files (starting with .) are exempt from case validation
  // They follow their own conventions: .gitignore, .env, .eslintrc.json, etc.
  if (isHiddenFile(name)) {
    return true;
  }

  switch (style) {
    case "kebab":
      return KEBAB_REGEX.test(name);
    case "snake":
      return SNAKE_REGEX.test(name);
    case "camel":
      return CAMEL_REGEX.test(name);
    case "pascal":
      return PASCAL_REGEX.test(name);
    case "any":
      return true;
    default:
      return true;
  }
};

const toKebab = (name: string): string => {
  const base = name.replace(/\.[^.]+$/, "");
  const ext = name.slice(base.length);
  const kebab = base
    .replace(/([a-z])([A-Z])/g, "$1-$2")
    .replace(/_/g, "-")
    .toLowerCase();
  return kebab + ext;
};

const toSnake = (name: string): string => {
  const base = name.replace(/\.[^.]+$/, "");
  const ext = name.slice(base.length);
  const snake = base
    .replace(/([a-z])([A-Z])/g, "$1_$2")
    .replace(/-/g, "_")
    .toLowerCase();
  return snake + ext;
};

const toCamel = (name: string): string => {
  const base = name.replace(/\.[^.]+$/, "");
  const ext = name.slice(base.length);
  const camel = base
    .replace(/[-_](.)/g, (_, c: string) => c.toUpperCase())
    .replace(/^(.)/, (c) => c.toLowerCase());
  return camel + ext;
};

const toPascal = (name: string): string => {
  const base = name.replace(/\.[^.]+$/, "");
  const ext = name.slice(base.length);
  const pascal = base
    .replace(/[-_](.)/g, (_, c: string) => c.toUpperCase())
    .replace(/^(.)/, (c) => c.toUpperCase());
  return pascal + ext;
};

export const suggestCase = (name: string, style: CaseStyle): string => {
  switch (style) {
    case "kebab":
      return toKebab(name);
    case "snake":
      return toSnake(name);
    case "camel":
      return toCamel(name);
    case "pascal":
      return toPascal(name);
    case "any":
      return name;
    default:
      return name;
  }
};

export const getCaseName = (style: CaseStyle): string => {
  switch (style) {
    case "kebab":
      return "kebab-case";
    case "snake":
      return "snake_case";
    case "camel":
      return "camelCase";
    case "pascal":
      return "PascalCase";
    case "any":
      return "any case";
    default:
      return style;
  }
};
