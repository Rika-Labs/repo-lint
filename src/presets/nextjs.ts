import type { RepoLintConfig, LayoutNode } from "../types.js";

const routeFiles: Record<string, LayoutNode> = {
  "page.tsx": { optional: true },
  "page.ts": { optional: true },
  "layout.tsx": { optional: true },
  "layout.ts": { optional: true },
  "loading.tsx": { optional: true },
  "error.tsx": { optional: true },
  "not-found.tsx": { optional: true },
  "template.tsx": { optional: true },
  "route.ts": { optional: true },
  "default.tsx": { optional: true },
};

export const nextjsPreset = (opts?: {
  readonly routeCase?: "kebab" | "snake" | "camel" | "pascal";
}): RepoLintConfig => {
  const routeCase = opts?.routeCase ?? "kebab";

  const layout: LayoutNode = {
    type: "dir",
    children: {
      src: {
        type: "dir",
        optional: true,
        children: {
          app: {
            type: "dir",
            children: {
              ...routeFiles,
              $routes: {
                type: "recursive",
                case: routeCase,
                optional: true,
                child: {
                  type: "dir",
                  children: routeFiles,
                },
              },
              api: {
                type: "dir",
                optional: true,
                children: {
                  $api: {
                    type: "many",
                    case: routeCase,
                    optional: true,
                    child: {
                      type: "dir",
                      children: {
                        "route.ts": {},
                      },
                    },
                  },
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
                  children: {
                    "index.tsx": {},
                    $files: { type: "many", pattern: "*.{ts,tsx,css}", optional: true },
                  },
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
        },
      },
      app: {
        type: "dir",
        optional: true,
        children: {
          ...routeFiles,
          $routes: {
            type: "recursive",
            case: routeCase,
            optional: true,
            child: { type: "dir", children: routeFiles },
          },
        },
      },
      public: {
        type: "dir",
        optional: true,
        children: { $assets: { type: "many", optional: true } },
      },
      "next.config.js": { optional: true },
      "next.config.mjs": { optional: true },
      "next.config.ts": { optional: true },
      "tailwind.config.js": { optional: true },
      "tailwind.config.ts": { optional: true },
      "tsconfig.json": { optional: true },
      "package.json": {},
    },
  };

  return {
    mode: "strict",
    layout,
    ignore: [".next", "node_modules", ".turbo", "out", ".vercel", "coverage", ".git"],
    rules: {
      ignorePaths: ["**/.next/**", "**/node_modules/**", "**/.turbo/**", "**/out/**"],
    },
  };
};
