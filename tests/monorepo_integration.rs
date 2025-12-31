use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn create_config(dir: &Path, content: &str) {
    fs::write(dir.join("repo-lint.config.ts"), content).unwrap();
}

fn create_minimal_config(dir: &Path) {
    let config = r#"
import { defineConfig, dir, file } from "@rikalabs/repo-lint";
export default defineConfig({ layout: dir({ "index.ts": file() }) });
"#;
    create_config(dir, config);
}

fn create_root_config_with_workspaces(dir: &Path) {
    let config = r#"
import { defineConfig, dir, file, opt, param } from "@rikalabs/repo-lint";

export default defineConfig({
    workspaces: ["apps/*", "packages/*"],
    layout: dir({
        apps: dir({
            $app: param({ case: "kebab" }, dir({
                "package.json": file(),
            })),
        }),
        packages: dir({
            $pkg: param({ case: "kebab" }, dir({
                "package.json": file(),
            })),
        }),
        "package.json": file(),
        "turbo.json": opt(file()),
    }),
});
"#;
    create_config(dir, config);
}

fn create_app_config(dir: &Path, _app_name: &str) {
    let config = r#"
import { defineConfig, dir, file, opt } from "@rikalabs/repo-lint";

export default defineConfig({
    layout: dir({
        src: dir({
            "index.ts": file(),
        }),
        "package.json": file(),
        "tsconfig.json": opt(file()),
    }),
});
"#;
    create_config(dir, config);
}

fn create_package_config(dir: &Path) {
    let config = r#"
import { defineConfig, dir, file, opt, many } from "@rikalabs/repo-lint";

export default defineConfig({
    layout: dir({
        src: dir({
            "index.ts": file(),
            $files: many(file("*.ts")),
        }),
        "package.json": file(),
    }),
});
"#;
    create_config(dir, config);
}

#[test]
fn test_discover_turborepo_style_monorepo() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    create_root_config_with_workspaces(root);
    fs::write(root.join("package.json"), "{}").unwrap();

    fs::create_dir_all(root.join("apps/web")).unwrap();
    create_app_config(&root.join("apps/web"), "web");
    fs::write(root.join("apps/web/package.json"), "{}").unwrap();

    fs::create_dir_all(root.join("apps/api")).unwrap();
    create_app_config(&root.join("apps/api"), "api");
    fs::write(root.join("apps/api/package.json"), "{}").unwrap();

    fs::create_dir_all(root.join("packages/ui")).unwrap();
    create_package_config(&root.join("packages/ui"));
    fs::write(root.join("packages/ui/package.json"), "{}").unwrap();

    let discovery = repo_lint::config::ConfigDiscovery::new(root);
    let configs = discovery.discover();

    assert_eq!(configs.len(), 4);

    let paths: Vec<&str> = configs.iter().map(|c| c.relative_path.as_str()).collect();
    assert!(paths.contains(&""));
    assert!(paths.contains(&"apps/api"));
    assert!(paths.contains(&"apps/web"));
    assert!(paths.contains(&"packages/ui"));
}

#[test]
fn test_discover_with_explicit_workspaces() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    create_root_config_with_workspaces(root);
    fs::write(root.join("package.json"), "{}").unwrap();

    fs::create_dir_all(root.join("apps/web")).unwrap();
    create_app_config(&root.join("apps/web"), "web");
    fs::write(root.join("apps/web/package.json"), "{}").unwrap();

    fs::create_dir_all(root.join("packages/ui")).unwrap();
    create_package_config(&root.join("packages/ui"));
    fs::write(root.join("packages/ui/package.json"), "{}").unwrap();

    fs::create_dir_all(root.join("tools/scripts")).unwrap();
    create_minimal_config(&root.join("tools/scripts"));

    let discovery = repo_lint::config::ConfigDiscovery::new(root);
    let configs =
        discovery.discover_with_workspaces(&["apps/*".to_string(), "packages/*".to_string()]);

    let paths: Vec<&str> = configs.iter().map(|c| c.relative_path.as_str()).collect();
    assert!(paths.contains(&"apps/web"));
    assert!(paths.contains(&"packages/ui"));
    assert!(!paths.contains(&"tools/scripts"));
}

#[test]
fn test_workspace_filter_exact() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    create_root_config_with_workspaces(root);
    fs::write(root.join("package.json"), "{}").unwrap();

    fs::create_dir_all(root.join("apps/web")).unwrap();
    create_app_config(&root.join("apps/web"), "web");
    fs::write(root.join("apps/web/package.json"), "{}").unwrap();

    fs::create_dir_all(root.join("apps/api")).unwrap();
    create_app_config(&root.join("apps/api"), "api");
    fs::write(root.join("apps/api/package.json"), "{}").unwrap();

    let discovery = repo_lint::config::ConfigDiscovery::new(root);
    let all_configs = discovery.discover();
    let filtered = discovery.filter_by_workspace(all_configs, "apps/web");

    let paths: Vec<&str> = filtered.iter().map(|c| c.relative_path.as_str()).collect();
    assert!(paths.contains(&"apps/web"));
    assert!(!paths.contains(&"apps/api"));
}

#[test]
fn test_workspace_filter_glob() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    create_root_config_with_workspaces(root);
    fs::write(root.join("package.json"), "{}").unwrap();

    fs::create_dir_all(root.join("apps/web")).unwrap();
    create_app_config(&root.join("apps/web"), "web");
    fs::write(root.join("apps/web/package.json"), "{}").unwrap();

    fs::create_dir_all(root.join("apps/api")).unwrap();
    create_app_config(&root.join("apps/api"), "api");
    fs::write(root.join("apps/api/package.json"), "{}").unwrap();

    fs::create_dir_all(root.join("packages/ui")).unwrap();
    create_package_config(&root.join("packages/ui"));
    fs::write(root.join("packages/ui/package.json"), "{}").unwrap();

    let discovery = repo_lint::config::ConfigDiscovery::new(root);
    let all_configs = discovery.discover();
    let filtered = discovery.filter_by_workspace(all_configs, "apps/*");

    let paths: Vec<&str> = filtered.iter().map(|c| c.relative_path.as_str()).collect();
    assert!(paths.contains(&"apps/web"));
    assert!(paths.contains(&"apps/api"));
    assert!(!paths.contains(&"packages/ui"));
}

#[test]
fn test_workspace_with_no_config_not_included() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    create_root_config_with_workspaces(root);
    fs::write(root.join("package.json"), "{}").unwrap();

    fs::create_dir_all(root.join("apps/web")).unwrap();
    create_app_config(&root.join("apps/web"), "web");
    fs::write(root.join("apps/web/package.json"), "{}").unwrap();

    fs::create_dir_all(root.join("apps/api")).unwrap();
    fs::write(root.join("apps/api/package.json"), "{}").unwrap();

    let discovery = repo_lint::config::ConfigDiscovery::new(root);
    let configs = discovery.discover();

    let paths: Vec<&str> = configs.iter().map(|c| c.relative_path.as_str()).collect();
    assert!(paths.contains(&"apps/web"));
    assert!(!paths.contains(&"apps/api"));
}

#[test]
fn test_nested_workspaces() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    create_minimal_config(root);
    fs::create_dir_all(root.join("apps/web/packages/shared")).unwrap();
    create_minimal_config(&root.join("apps/web"));
    create_minimal_config(&root.join("apps/web/packages/shared"));

    let discovery = repo_lint::config::ConfigDiscovery::new(root);
    let configs = discovery.discover();

    assert_eq!(configs.len(), 3);

    let paths: Vec<&str> = configs.iter().map(|c| c.relative_path.as_str()).collect();
    assert!(paths.contains(&""));
    assert!(paths.contains(&"apps/web"));
    assert!(paths.contains(&"apps/web/packages/shared"));
}

#[test]
fn test_parse_workspaces_field() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let config = r#"
import { defineConfig, dir, file } from "@rikalabs/repo-lint";

export default defineConfig({
    workspaces: ["apps/*", "packages/*"],
    layout: dir({ "package.json": file() }),
});
"#;
    create_config(root, config);

    let parser = repo_lint::config::ConfigParser::new();
    let result = parser.parse_file(&root.join("repo-lint.config.ts"));

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.workspaces.len(), 2);
    assert!(config.workspaces.contains(&"apps/*".to_string()));
    assert!(config.workspaces.contains(&"packages/*".to_string()));
}

#[test]
fn test_parse_empty_workspaces() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let config = r#"
import { defineConfig, dir, file } from "@rikalabs/repo-lint";

export default defineConfig({
    workspaces: [],
    layout: dir({ "package.json": file() }),
});
"#;
    create_config(root, config);

    let parser = repo_lint::config::ConfigParser::new();
    let result = parser.parse_file(&root.join("repo-lint.config.ts"));

    assert!(result.is_ok());
    let config = result.unwrap();
    assert!(config.workspaces.is_empty());
}

#[test]
fn test_config_without_workspaces_defaults_to_empty() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    create_minimal_config(root);

    let parser = repo_lint::config::ConfigParser::new();
    let result = parser.parse_file(&root.join("repo-lint.config.ts"));

    assert!(result.is_ok());
    let config = result.unwrap();
    assert!(config.workspaces.is_empty());
}

#[test]
fn test_single_workspace_pattern() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let config = r#"
import { defineConfig, dir, file } from "@rikalabs/repo-lint";

export default defineConfig({
    workspaces: ["packages/*"],
    layout: dir({ "package.json": file() }),
});
"#;
    create_config(root, config);

    let parser = repo_lint::config::ConfigParser::new();
    let result = parser.parse_file(&root.join("repo-lint.config.ts"));

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.workspaces.len(), 1);
    assert_eq!(config.workspaces[0], "packages/*");
}

#[test]
fn test_multiple_workspace_patterns() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let config = r#"
import { defineConfig, dir, file } from "@rikalabs/repo-lint";

export default defineConfig({
    workspaces: ["apps/*", "packages/*", "libs/*", "tools/*"],
    layout: dir({ "package.json": file() }),
});
"#;
    create_config(root, config);

    let parser = repo_lint::config::ConfigParser::new();
    let result = parser.parse_file(&root.join("repo-lint.config.ts"));

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.workspaces.len(), 4);
}

#[test]
fn test_discover_configs_sorted_by_path() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    create_minimal_config(root);
    fs::create_dir_all(root.join("z-last")).unwrap();
    create_minimal_config(&root.join("z-last"));
    fs::create_dir_all(root.join("a-first")).unwrap();
    create_minimal_config(&root.join("a-first"));
    fs::create_dir_all(root.join("m-middle")).unwrap();
    create_minimal_config(&root.join("m-middle"));

    let discovery = repo_lint::config::ConfigDiscovery::new(root);
    let configs = discovery.discover();

    let paths: Vec<&str> = configs.iter().map(|c| c.relative_path.as_str()).collect();
    assert_eq!(paths[0], "");
    assert_eq!(paths[1], "a-first");
    assert_eq!(paths[2], "m-middle");
    assert_eq!(paths[3], "z-last");
}

#[test]
fn test_find_root_config_when_exists() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    create_minimal_config(root);

    let discovery = repo_lint::config::ConfigDiscovery::new(root);
    let root_config = discovery.find_root_config();

    assert!(root_config.is_some());
    assert_eq!(root_config.unwrap().relative_path, "");
}

#[test]
fn test_find_root_config_when_missing() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let discovery = repo_lint::config::ConfigDiscovery::new(root);
    let root_config = discovery.find_root_config();

    assert!(root_config.is_none());
}

#[test]
fn test_pnpm_workspace_style() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let config = r#"
import { defineConfig, dir, file, opt, param } from "@rikalabs/repo-lint";

export default defineConfig({
    workspaces: ["packages/*"],
    layout: dir({
        packages: dir({
            $pkg: param({ case: "kebab" }, dir({
                src: opt(dir({})),
                "package.json": file(),
            })),
        }),
        "package.json": file(),
        "pnpm-workspace.yaml": opt(file()),
    }),
});
"#;
    create_config(root, config);
    fs::write(root.join("package.json"), "{}").unwrap();
    fs::write(
        root.join("pnpm-workspace.yaml"),
        "packages:\n  - packages/*\n",
    )
    .unwrap();

    fs::create_dir_all(root.join("packages/utils")).unwrap();
    create_minimal_config(&root.join("packages/utils"));
    fs::write(root.join("packages/utils/package.json"), "{}").unwrap();

    let parser = repo_lint::config::ConfigParser::new();
    let result = parser.parse_file(&root.join("repo-lint.config.ts"));

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.workspaces, vec!["packages/*"]);

    let discovery = repo_lint::config::ConfigDiscovery::new(root);
    let configs = discovery.discover_with_workspaces(&config.workspaces);

    assert_eq!(configs.len(), 2);
}

#[test]
fn test_nx_style_libs_and_apps() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let config = r#"
import { defineConfig, dir, file, opt, param } from "@rikalabs/repo-lint";

export default defineConfig({
    workspaces: ["apps/*", "libs/*"],
    layout: dir({
        apps: dir({
            $app: param({ case: "kebab" }, dir({
                "project.json": opt(file()),
            })),
        }),
        libs: dir({
            $lib: param({ case: "kebab" }, dir({
                "project.json": opt(file()),
            })),
        }),
        "nx.json": opt(file()),
    }),
});
"#;
    create_config(root, config);

    fs::create_dir_all(root.join("apps/my-app")).unwrap();
    create_minimal_config(&root.join("apps/my-app"));

    fs::create_dir_all(root.join("libs/shared")).unwrap();
    create_minimal_config(&root.join("libs/shared"));

    let discovery = repo_lint::config::ConfigDiscovery::new(root);
    let configs = discovery.discover();

    let paths: Vec<&str> = configs.iter().map(|c| c.relative_path.as_str()).collect();
    assert!(paths.contains(&"apps/my-app"));
    assert!(paths.contains(&"libs/shared"));
}

#[test]
fn test_workspace_package_import_resolution() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create root package.json with workspaces
    fs::write(
        root.join("package.json"),
        r#"{"name": "root", "workspaces": ["packages/*", "apps/*"]}"#,
    )
    .unwrap();

    // Create shared config package
    fs::create_dir_all(root.join("packages/config/repo-lint")).unwrap();
    fs::write(
        root.join("packages/config/package.json"),
        r#"{"name": "@myorg/config"}"#,
    )
    .unwrap();

    // Create shared layout in config package
    let shared_config = r#"
import { defineConfig, dir, file, many, opt } from "@rikalabs/repo-lint";

export const sharedLayout = dir({
    src: dir({
        "index.ts": file(),
        $files: many(file("*.ts")),
    }),
    "package.json": file(),
});

export const sharedIgnore = ["node_modules", "dist"];
"#;
    fs::write(
        root.join("packages/config/repo-lint/shared.ts"),
        shared_config,
    )
    .unwrap();

    // Create app that imports from shared config
    fs::create_dir_all(root.join("apps/web/src")).unwrap();
    fs::write(
        root.join("apps/web/package.json"),
        r#"{"name": "@myorg/web"}"#,
    )
    .unwrap();
    fs::write(root.join("apps/web/src/index.ts"), "export {}").unwrap();

    let app_config = r#"
import { defineConfig } from "@rikalabs/repo-lint";
import { sharedLayout, sharedIgnore } from "@myorg/config/repo-lint/shared";

export default defineConfig({
    ignore: sharedIgnore,
    layout: sharedLayout,
});
"#;
    fs::write(root.join("apps/web/repo-lint.config.ts"), app_config).unwrap();

    // First verify the import path resolution works
    let parser = repo_lint::config::ConfigParser::new();
    let import_path = parser.resolve_import(
        &root.join("apps/web/repo-lint.config.ts"),
        "@myorg/config/repo-lint/shared",
    );
    assert!(
        import_path.is_some(),
        "Failed to resolve workspace package import"
    );

    // Parse the app config - it should resolve the workspace package import
    let result = parser.parse_file(&root.join("apps/web/repo-lint.config.ts"));

    assert!(result.is_ok(), "Failed to parse config: {:?}", result.err());
    let config = result.unwrap();
    assert!(config.layout.is_some());
    assert_eq!(config.ignore, vec!["node_modules", "dist"]);
}
