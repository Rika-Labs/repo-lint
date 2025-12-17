use repo_lint::config::{ConfigParser, Mode};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_extends_relative_path() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let base_config = r#"
import { defineConfig, dir, file } from "repo-lint";
export const sharedLayout = dir({ "README.md": file() });
export default defineConfig({
    mode: "warn",
    layout: sharedLayout,
    rules: { forbidPaths: ["**/tmp/**"] }
});
"#;
    fs::write(root.join("base.config.ts"), base_config).unwrap();

    let child_config = r#"
import { defineConfig, dir, file } from "repo-lint";
export default defineConfig({
    extends: "./base.config.ts",
    layout: dir({ "index.ts": file() }),
    rules: { forbidPaths: ["**/dist/**"] }
});
"#;
    let child_path = root.join("child.config.ts");
    fs::write(&child_path, child_config).unwrap();

    let parser = ConfigParser::new();
    let config = parser.parse_file(&child_path).unwrap();

    assert_eq!(config.mode, Mode::Warn);
    assert_eq!(config.rules.forbid_paths.len(), 2);
    assert!(config.rules.forbid_paths.contains(&"**/tmp/**".to_string()));
    assert!(config
        .rules
        .forbid_paths
        .contains(&"**/dist/**".to_string()));
}

#[test]
fn test_extends_root_alias() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let root_config = r#"
import { defineConfig, dir, file } from "repo-lint";
export default defineConfig({
    mode: "warn",
    layout: dir({ "root.ts": file() }),
    rules: { forbidPaths: ["**/root/**"] }
});
"#;
    fs::write(root.join("repo-lint.config.ts"), root_config).unwrap();

    let apps_dir = root.join("apps/web");
    fs::create_dir_all(&apps_dir).unwrap();

    let child_config = r#"
import { defineConfig, dir, file } from "repo-lint";
export default defineConfig({
    extends: "@/repo-lint.config.ts",
    layout: dir({ "app.ts": file() }),
    rules: { forbidPaths: ["**/app/**"] }
});
"#;
    let child_path = apps_dir.join("repo-lint.config.ts");
    fs::write(&child_path, child_config).unwrap();

    let parser = ConfigParser::new();
    let config = parser.parse_file(&child_path).unwrap();

    assert_eq!(config.mode, Mode::Warn);
    assert_eq!(config.rules.forbid_paths.len(), 2);
    assert!(config
        .rules
        .forbid_paths
        .contains(&"**/root/**".to_string()));
    assert!(config.rules.forbid_paths.contains(&"**/app/**".to_string()));
}

#[test]
fn test_import_scoped_package() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let pkg_dir = root.join("node_modules/@intimetec/config/repo-lint");
    fs::create_dir_all(&pkg_dir).unwrap();

    let shared_config = r#"
import { dir, file } from "repo-lint";
export const baseLayout = dir({ "package.json": file() });
"#;
    fs::write(pkg_dir.join("nextjs.ts"), shared_config).unwrap();

    let config_content = r#"
import { defineConfig, dir } from "repo-lint";
import { baseLayout } from "@intimetec/config/repo-lint/nextjs";

export default defineConfig({
    layout: dir({
        apps: dir({
            $app: baseLayout,
        })
    })
});
"#;
    let config_path = root.join("repo-lint.config.ts");
    fs::write(&config_path, config_content).unwrap();

    let parser = ConfigParser::new();
    let config = parser.parse_file(&config_path).unwrap();

    if let Some(repo_lint::config::LayoutNode::Dir { children, .. }) = &config.layout {
        assert!(children.contains_key("apps"));
    } else {
        panic!("Expected root to be a directory");
    }
}

#[test]
fn test_nextjs_preset() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let config_content = r#"
import { defineConfig, nextjsPreset } from "repo-lint";

export default defineConfig(nextjsPreset({
    routeCase: "kebab"
}));
"#;
    let config_path = root.join("repo-lint.config.ts");
    fs::write(&config_path, config_content).unwrap();

    let parser = ConfigParser::new();
    let config = parser.parse_file(&config_path).unwrap();

    // Verify it has the app directory structure
    if let Some(repo_lint::config::LayoutNode::Dir { children, .. }) = &config.layout {
        assert!(children.contains_key("app"));
    } else {
        panic!("Expected root to be a directory");
    }
}
