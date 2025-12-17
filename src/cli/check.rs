use clap::Args;
use rayon::prelude::*;
use std::path::{Path, PathBuf};

use crate::config::{ConfigDiscovery, ConfigParser, DiscoveredConfig};
use crate::engine::{FileMatcher, PostValidator, Severity, Violation, Walker};
use crate::output::{create_reporter, OutputFormat};

#[derive(Args)]
pub struct CheckArgs {
    #[arg(default_value = ".")]
    pub path: PathBuf,

    #[arg(long)]
    pub changed: bool,

    #[arg(long, default_value = "HEAD")]
    pub base: String,

    #[arg(long)]
    pub fix: bool,

    #[arg(
        long,
        help = "Only validate paths under this scope (e.g., apps/sentinel)"
    )]
    pub scope: Option<String>,
}

#[derive(Debug)]
pub struct WorkspaceResult {
    pub workspace: String,
    pub config_path: PathBuf,
    pub violations: Vec<Violation>,
}

pub struct CheckCommand;

impl CheckCommand {
    pub fn run(
        args: &CheckArgs,
        config_path: &str,
        output_format: OutputFormat,
        _agent_mode: bool,
        _trace_mode: bool,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        Self::run_with_workspace(args, config_path, output_format, None)
    }

    pub fn run_with_workspace(
        args: &CheckArgs,
        config_path: &str,
        output_format: OutputFormat,
        workspace_filter: Option<&str>,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let config_file = PathBuf::from(config_path);

        if config_file.exists() {
            let parser = ConfigParser::new();
            let config = parser.parse_file(&config_file)?;

            if !config.workspaces.is_empty() {
                return Self::run_monorepo(args, &config_file, output_format, workspace_filter);
            }

            return Self::run_single_config(args, &config_file, output_format);
        }

        let discovery = ConfigDiscovery::new(&args.path);
        let configs = discovery.discover();

        if configs.is_empty() {
            return Err("No repo-lint.config.ts found".into());
        }

        if configs.len() == 1 {
            return Self::run_single_config(args, &configs[0].config_path, output_format);
        }

        Self::run_multi_config(args, configs, output_format, workspace_filter)
    }

    fn run_monorepo(
        args: &CheckArgs,
        root_config_path: &Path,
        output_format: OutputFormat,
        workspace_filter: Option<&str>,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let parser = ConfigParser::new();
        let root_config = parser.parse_file(root_config_path)?;

        let root_dir = root_config_path.parent().unwrap_or(Path::new("."));
        let discovery = ConfigDiscovery::new(root_dir);

        let mut configs = discovery.discover_with_workspaces(&root_config.workspaces);

        if let Some(filter) = workspace_filter {
            configs = discovery.filter_by_workspace(configs, filter);
        }

        Self::run_multi_config(args, configs, output_format, workspace_filter)
    }

    fn run_multi_config(
        args: &CheckArgs,
        configs: Vec<DiscoveredConfig>,
        output_format: OutputFormat,
        _workspace_filter: Option<&str>,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let parser = ConfigParser::new();
        let mut workspace_results: Vec<WorkspaceResult> = Vec::new();

        for discovered in &configs {
            let workspace_name = if discovered.relative_path.is_empty() {
                ".".to_string()
            } else {
                discovered.relative_path.clone()
            };

            match parser.parse_file(&discovered.config_path) {
                Ok(config) => {
                    let violations = Self::check_workspace(
                        args,
                        &discovered.workspace_root,
                        &config,
                        &discovered.relative_path,
                    )?;

                    workspace_results.push(WorkspaceResult {
                        workspace: workspace_name,
                        config_path: discovered.config_path.clone(),
                        violations,
                    });
                }
                Err(e) => {
                    eprintln!(
                        "Error parsing config {}: {}",
                        discovered.config_path.display(),
                        e
                    );
                }
            }
        }

        Self::report_workspace_results(&workspace_results, output_format)
    }

    fn check_workspace(
        args: &CheckArgs,
        workspace_root: &Path,
        config: &crate::config::ConfigIR,
        workspace_prefix: &str,
    ) -> Result<Vec<Violation>, Box<dyn std::error::Error>> {
        let matcher = FileMatcher::new(config)?;
        let mut walker = Walker::new(workspace_root).respect_gitignore(config.use_gitignore);

        for ignore_pattern in &config.ignore {
            walker = walker.add_ignore(ignore_pattern);
        }

        let scope_prefix = args.scope.as_ref().map(|s| {
            let mut p = s.clone();
            if !p.ends_with('/') {
                p.push('/');
            }
            p
        });

        let strict_mode = matches!(config.mode, crate::config::Mode::Strict);
        let severity = if strict_mode {
            Severity::Error
        } else {
            Severity::Warning
        };

        let mut post_validator = PostValidator::new(config);
        let mut violations: Vec<Violation> = if args.changed {
            let entries = walker.walk_changed(&args.base)?;
            for entry in &entries {
                post_validator.record_path(&entry.relative_path);
            }
            entries
                .par_iter()
                .filter(|entry| {
                    if let Some(ref prefix) = scope_prefix {
                        entry.relative_path.to_string_lossy().starts_with(prefix)
                            || entry.relative_path.to_string_lossy() == prefix.trim_end_matches('/')
                    } else {
                        true
                    }
                })
                .flat_map(|entry| matcher.check_path(&entry.relative_path))
                .collect()
        } else {
            let all_paths = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
            let all_paths_clone = all_paths.clone();
            let v = walker.walk_and_process(|path| {
                all_paths_clone.lock().unwrap().push(path.to_path_buf());
                if let Some(ref prefix) = scope_prefix {
                    let path_str = path.to_string_lossy();
                    if !path_str.starts_with(prefix) && path_str != prefix.trim_end_matches('/') {
                        return Vec::new();
                    }
                }
                matcher.check_path(path)
            });
            for path in all_paths.lock().unwrap().iter() {
                post_validator.record_path(path);
            }
            v
        };

        violations.extend(post_validator.validate(workspace_root, severity));

        let violations = violations
            .into_iter()
            .map(|mut v| {
                if !workspace_prefix.is_empty() {
                    v.path = format!("{}/{}", workspace_prefix, v.path).into();
                }
                v
            })
            .collect();

        Ok(violations)
    }

    fn run_single_config(
        args: &CheckArgs,
        config_path: &Path,
        output_format: OutputFormat,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let parser = ConfigParser::new();
        let config = parser.parse_file(config_path)?;

        let matcher = FileMatcher::new(&config)?;
        let mut walker = Walker::new(&args.path).respect_gitignore(config.use_gitignore);

        for ignore_pattern in &config.ignore {
            walker = walker.add_ignore(ignore_pattern);
        }

        let scope_prefix = args.scope.as_ref().map(|s| {
            let mut p = s.clone();
            if !p.ends_with('/') {
                p.push('/');
            }
            p
        });

        let strict_mode = matches!(config.mode, crate::config::Mode::Strict);
        let severity = if strict_mode {
            Severity::Error
        } else {
            Severity::Warning
        };

        let mut post_validator = PostValidator::new(&config);
        let mut all_violations: Vec<Violation> = if args.changed {
            let entries = walker.walk_changed(&args.base)?;
            for entry in &entries {
                post_validator.record_path(&entry.relative_path);
            }
            entries
                .par_iter()
                .filter(|entry| {
                    if let Some(ref prefix) = scope_prefix {
                        entry.relative_path.to_string_lossy().starts_with(prefix)
                            || entry.relative_path.to_string_lossy() == prefix.trim_end_matches('/')
                    } else {
                        true
                    }
                })
                .flat_map(|entry| matcher.check_path(&entry.relative_path))
                .collect()
        } else {
            let all_paths = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
            let all_paths_clone = all_paths.clone();
            let violations = walker.walk_and_process(|path| {
                all_paths_clone.lock().unwrap().push(path.to_path_buf());
                if let Some(ref prefix) = scope_prefix {
                    let path_str = path.to_string_lossy();
                    if !path_str.starts_with(prefix) && path_str != prefix.trim_end_matches('/') {
                        return Vec::new();
                    }
                }
                matcher.check_path(path)
            });
            for path in all_paths.lock().unwrap().iter() {
                post_validator.record_path(path);
            }
            violations
        };

        all_violations.extend(post_validator.validate(&args.path, severity));
        all_violations.sort_by(|a, b| a.path.cmp(&b.path).then_with(|| a.rule_id.cmp(&b.rule_id)));

        let reporter = create_reporter(output_format);
        let output = reporter.report(&all_violations);
        println!("{}", output);

        let has_errors = all_violations.iter().any(|v| v.severity == Severity::Error);

        Ok(if has_errors { 1 } else { 0 })
    }

    fn report_workspace_results(
        results: &[WorkspaceResult],
        output_format: OutputFormat,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let reporter = create_reporter(output_format);

        let mut all_violations: Vec<Violation> = Vec::new();
        let mut has_errors = false;

        for result in results {
            if !result.violations.is_empty() {
                println!("\n[{}]", result.workspace);
                let output = reporter.report(&result.violations);
                println!("{}", output);

                if result
                    .violations
                    .iter()
                    .any(|v| v.severity == Severity::Error)
                {
                    has_errors = true;
                }

                all_violations.extend(result.violations.clone());
            }
        }

        let total_errors = all_violations
            .iter()
            .filter(|v| v.severity == Severity::Error)
            .count();
        let total_warnings = all_violations
            .iter()
            .filter(|v| v.severity == Severity::Warning)
            .count();
        let workspaces_with_violations =
            results.iter().filter(|r| !r.violations.is_empty()).count();

        if total_errors > 0 || total_warnings > 0 {
            println!(
                "\nSummary: {} error(s) and {} warning(s) across {} workspace(s)",
                total_errors, total_warnings, workspaces_with_violations
            );
        } else {
            let total_workspaces = results.len();
            println!(
                "\nNo violations found across {} workspace(s).",
                total_workspaces
            );
        }

        Ok(if has_errors { 1 } else { 0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_config(dir: &std::path::Path) -> PathBuf {
        let config_content = r#"
import { defineConfig, dir, file } from "repo-lint";

export default defineConfig({
    mode: "strict",
    layout: dir({
        src: dir({
            "index.ts": file(),
        }),
    }),
    rules: {
        forbidPaths: ["**/utils/**"],
        forbidNames: ["temp"],
    },
});
"#;
        let config_path = dir.join("repo-lint.config.ts");
        fs::write(&config_path, config_content).unwrap();
        config_path
    }

    #[test]
    fn test_check_command_valid_structure() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/index.ts"), "").unwrap();

        let config_path = create_test_config(root);

        let args = CheckArgs {
            path: root.to_path_buf(),
            changed: false,
            base: "HEAD".to_string(),
            fix: false,
            scope: None,
        };

        let result = CheckCommand::run(
            &args,
            config_path.to_str().unwrap(),
            OutputFormat::Console,
            false,
            false,
        );

        assert!(result.is_ok());
    }
}
