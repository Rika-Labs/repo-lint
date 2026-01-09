import type { RepoLintConfig, LayoutNode, CaseStyle } from "../../types/index.js";

type NextjsPresetOptions = {
  readonly routeCase?: CaseStyle;
};

export const nextjsPreset = (options: NextjsPresetOptions = {}): RepoLintConfig => {
  const routeCase = options.routeCase ?? "kebab";

  const routeChildren: Record<string, LayoutNode> = {
    "page.tsx": { optional: true },
    "layout.tsx": { optional: true },
    "loading.tsx": { optional: true },
    "error.tsx": { optional: true },
    "not-found.tsx": { optional: true },
    "template.tsx": { optional: true },
    "default.tsx": { optional: true },
    "route.ts": { optional: true },
    $route: {
      type: "param",
      case: routeCase,
      child: {
        type: "dir",
        children: {
          "page.tsx": { optional: true },
          "layout.tsx": { optional: true },
          "loading.tsx": { optional: true },
          "error.tsx": { optional: true },
          $files: { type: "many", pattern: "*.{ts,tsx,css}", optional: true },
        },
      },
    },
  };

  const layout: LayoutNode = {
    type: "dir",
    children: {
      app: {
        type: "dir",
        optional: true,
        children: {
          ...routeChildren,
          $dynamic: {
            type: "many",
            pattern: "\\[*\\]",
            child: {
              type: "dir",
              children: routeChildren,
            },
          },
        },
      },
      components: {
        type: "dir",
        optional: true,
        children: {
          $component: {
            type: "many",
            case: "pascal",
            child: {
              type: "dir",
              children: { $files: { type: "many", pattern: "*.{ts,tsx,css}" } },
            },
          },
        },
      },
      lib: {
        type: "dir",
        optional: true,
        children: { $files: { type: "many", pattern: "*.ts" } },
      },
      hooks: {
        type: "dir",
        optional: true,
        children: { $hook: { type: "many", case: "camel", pattern: "use*.ts" } },
      },
      public: {
        type: "dir",
        optional: true,
        children: { $assets: { type: "many", optional: true } },
      },
      "package.json": { required: true },
      "next.config.js": { optional: true },
      "next.config.mjs": { optional: true },
      "next.config.ts": { optional: true },
      "tsconfig.json": { optional: true },
      ".env": { optional: true },
      ".env.local": { optional: true },
    },
  };

  return {
    mode: "strict",
    ignore: [
      "node_modules/**",
      ".next/**",
      ".git/**",
      "dist/**",
      "out/**",
      "coverage/**",
    ],
    useGitignore: true,
    layout,
    rules: {
      forbidPaths: [
        "**/node_modules/**",
      ],
      forbidNames: [
        ".DS_Store",
        "Thumbs.db",
      ],
    },
  };
};
