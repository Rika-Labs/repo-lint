use clap::Args;
use rayon::prelude::*;
use std::path::PathBuf;

use crate::config::ConfigParser;
use crate::engine::{FileMatcher, Violation, Walker};
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

pub struct CheckCommand;

impl CheckCommand {
    pub fn run(
        args: &CheckArgs,
        config_path: &str,
        output_format: OutputFormat,
        _agent_mode: bool,
        _trace_mode: bool,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let config_file = PathBuf::from(config_path);
        if !config_file.exists() {
            return Err(format!("Config file not found: {}", config_path).into());
        }

        let parser = ConfigParser::new();
        let config = parser.parse_file(&config_file)?;

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

        let mut all_violations: Vec<Violation> = if args.changed {
            let entries = walker.walk_changed(&args.base)?;
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
            walker.walk_and_process(|path| {
                if let Some(ref prefix) = scope_prefix {
                    let path_str = path.to_string_lossy();
                    if !path_str.starts_with(prefix) && path_str != prefix.trim_end_matches('/') {
                        return Vec::new();
                    }
                }
                matcher.check_path(path)
            })
        };

        all_violations.sort_by(|a, b| a.path.cmp(&b.path).then_with(|| a.rule_id.cmp(&b.rule_id)));

        let reporter = create_reporter(output_format);
        let output = reporter.report(&all_violations);
        println!("{}", output);

        let has_errors = all_violations
            .iter()
            .any(|v| v.severity == crate::engine::Severity::Error);

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
