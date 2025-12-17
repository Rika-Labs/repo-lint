use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

use repo_lint::cli::{CheckArgs, CheckCommand};
use repo_lint::output::OutputFormat;

fn create_config(dir: &std::path::Path, content: &str) -> PathBuf {
    let config_path = dir.join("repo-lint.config.ts");
    fs::write(&config_path, content).unwrap();
    config_path
}

fn basic_config() -> &'static str {
    r#"
import { defineConfig, dir, file, opt, param } from "repo-lint";

export default defineConfig({
    mode: "strict",
    layout: dir({
        src: dir({
            "index.ts": file(),
            services: opt(dir({
                $module: param({ case: "kebab" }, dir({
                    "index.ts": file(),
                })),
            })),
        }),
        "README.md": opt(file()),
    }),
    rules: {
        forbidPaths: ["**/utils/**", "**/*.bak"],
        forbidNames: ["temp", "new"],
    },
});
"#
}

#[test]
fn test_check_empty_directory() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/index.ts"), "").unwrap();

    let config_path = create_config(root, basic_config());

    let args = CheckArgs {
        path: root.to_path_buf(),
        changed: false,
        base: "HEAD".to_string(),
        fix: false,
    };

    let result = CheckCommand::run(
        &args,
        config_path.to_str().unwrap(),
        OutputFormat::Json,
        false,
        false,
    );

    assert!(result.is_ok());
}

#[test]
fn test_check_valid_module_structure() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::create_dir_all(root.join("src/services/billing")).unwrap();
    fs::write(root.join("src/index.ts"), "").unwrap();
    fs::write(root.join("src/services/billing/index.ts"), "").unwrap();

    let config_path = create_config(root, basic_config());

    let args = CheckArgs {
        path: root.to_path_buf(),
        changed: false,
        base: "HEAD".to_string(),
        fix: false,
    };

    let result = CheckCommand::run(
        &args,
        config_path.to_str().unwrap(),
        OutputFormat::Json,
        false,
        false,
    );

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_check_forbidden_path_violation() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::create_dir_all(root.join("src/utils")).unwrap();
    fs::write(root.join("src/index.ts"), "").unwrap();
    fs::write(root.join("src/utils/helper.ts"), "").unwrap();

    let config_path = create_config(root, basic_config());

    let args = CheckArgs {
        path: root.to_path_buf(),
        changed: false,
        base: "HEAD".to_string(),
        fix: false,
    };

    let result = CheckCommand::run(
        &args,
        config_path.to_str().unwrap(),
        OutputFormat::Json,
        false,
        false,
    );

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);
}

#[test]
fn test_check_forbidden_name_violation() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/index.ts"), "").unwrap();
    fs::write(root.join("src/temp.ts"), "").unwrap();

    let config_path = create_config(root, basic_config());

    let args = CheckArgs {
        path: root.to_path_buf(),
        changed: false,
        base: "HEAD".to_string(),
        fix: false,
    };

    let result = CheckCommand::run(
        &args,
        config_path.to_str().unwrap(),
        OutputFormat::Json,
        false,
        false,
    );

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);
}

#[test]
fn test_check_invalid_case_violation() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::create_dir_all(root.join("src/services/MyModule")).unwrap();
    fs::write(root.join("src/index.ts"), "").unwrap();
    fs::write(root.join("src/services/MyModule/index.ts"), "").unwrap();

    let config_path = create_config(root, basic_config());

    let args = CheckArgs {
        path: root.to_path_buf(),
        changed: false,
        base: "HEAD".to_string(),
        fix: false,
    };

    let result = CheckCommand::run(
        &args,
        config_path.to_str().unwrap(),
        OutputFormat::Json,
        false,
        false,
    );

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);
}

#[test]
fn test_check_sarif_output() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/index.ts"), "").unwrap();

    let config_path = create_config(root, basic_config());

    let args = CheckArgs {
        path: root.to_path_buf(),
        changed: false,
        base: "HEAD".to_string(),
        fix: false,
    };

    let result = CheckCommand::run(
        &args,
        config_path.to_str().unwrap(),
        OutputFormat::Sarif,
        false,
        false,
    );

    assert!(result.is_ok());
}

#[test]
fn test_check_missing_config() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let args = CheckArgs {
        path: root.to_path_buf(),
        changed: false,
        base: "HEAD".to_string(),
        fix: false,
    };

    let result = CheckCommand::run(
        &args,
        "nonexistent.config.ts",
        OutputFormat::Console,
        false,
        false,
    );

    assert!(result.is_err());
}
