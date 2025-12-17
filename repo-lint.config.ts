import { defineConfig, dir, file, opt, param, many } from "repo-lint";

export default defineConfig({
  mode: "strict",

  layout: dir({
    ".github": opt(dir({
      workflows: opt(dir({
        $workflow: many(file("*.yml")),
      })),
    })),

    benches: opt(dir({
      $bench: many(file("*.rs")),
    })),

    docs: opt(dir({
      src: dir({
        cli: dir({
          $doc: many(file("*.md")),
        }),
        configuration: dir({
          $doc: many(file("*.md")),
        }),
        contributing: dir({
          $doc: many(file("*.md")),
        }),
        examples: dir({
          $doc: many(file("*.md")),
        }),
        "getting-started": dir({
          $doc: many(file("*.md")),
        }),
        "introduction.md": file(),
        "SUMMARY.md": file(),
      }),
      "book.toml": opt(file()),
    })),

    examples: opt(dir({
      $example: many(file("*.rs")),
    })),

    npm: opt(dir({
      bin: dir({
        "repo-lint": file(),
      }),
      types: dir({
        "index.d.ts": file(),
      }),
      "package.json": file(),
      "index.js": file(),
      "install.js": file(),
      "README.md": opt(file()),
    })),

    src: dir({
      cache: dir({
        "mod.rs": file(),
        $module: many(file("*.rs")),
      }),
      cli: dir({
        "mod.rs": file(),
        $command: many(file("*.rs")),
      }),
      config: dir({
        "mod.rs": file(),
        $module: many(file("*.rs")),
      }),
      engine: dir({
        "mod.rs": file(),
        $module: many(file("*.rs")),
      }),
      output: dir({
        "mod.rs": file(),
        $formatter: many(file("*.rs")),
      }),
      "lib.rs": file(),
      "main.rs": file(),
    }),

    tests: opt(dir({
      integration: opt(dir({
        "mod.rs": file(),
        $test: many(file("*.rs")),
      })),
    })),

    "Cargo.toml": file(),
    "Cargo.lock": file(),
    "LICENSE": file(),
    "README.md": file(),
    "CHANGELOG.md": opt(file()),
    "CONTRIBUTING.md": opt(file()),
    ".gitignore": file(),
    "repo-lint.config.ts": file(),
  }),

  rules: {
    forbidPaths: ["**/target/**", "**/*.bak", "**/*~"],
    forbidNames: ["tmp", "temp", "new", "old", "copy"],
  },
});
