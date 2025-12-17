use compact_str::CompactString;
use std::path::Path;

use super::layout_trie::{LayoutMatcher, MatchAttempt, MatchResult};
use super::rules::CompiledRules;
use crate::config::ConfigIR;

#[derive(Debug, Clone)]
pub struct Violation {
    pub path: CompactString,
    pub rule_id: CompactString,
    pub message: CompactString,
    pub severity: Severity,
    pub fix_suggestion: Option<CompactString>,
    pub attempts: Vec<MatchAttempt>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
        }
    }
}

pub struct FileMatcher {
    layout_matcher: LayoutMatcher,
    rules: CompiledRules,
    strict_mode: bool,
}

impl FileMatcher {
    pub fn new(config: &ConfigIR) -> Result<Self, globset::Error> {
        let layout_matcher = LayoutMatcher::new(config.layout.clone());
        let rules = CompiledRules::compile(&config.rules)?;
        let strict_mode = matches!(config.mode, crate::config::Mode::Strict);

        Ok(Self {
            layout_matcher,
            rules,
            strict_mode,
        })
    }

    pub fn is_ignored(&self, path: &Path) -> bool {
        self.rules.is_ignored(path)
    }

    pub fn check_path(&self, path: &Path) -> Vec<Violation> {
        if self.rules.is_ignored(path) {
            return Vec::new();
        }

        let mut violations = Vec::new();
        let severity = if self.strict_mode {
            Severity::Error
        } else {
            Severity::Warning
        };

        let path_str = CompactString::new(path.to_string_lossy());

        let rule_violations = self.rules.check_path(path);
        for rv in rule_violations {
            violations.push(Violation {
                path: path_str.clone(),
                rule_id: CompactString::const_new(rv.rule_id()),
                message: CompactString::new(rv.message()),
                severity,
                fix_suggestion: None,
                attempts: Vec::new(),
            });
        }

        let match_result = self.layout_matcher.match_path(path);
        match match_result {
            MatchResult::Allowed
            | MatchResult::AllowedParam { .. }
            | MatchResult::AllowedMany { .. } => {}
            MatchResult::Denied { reason, attempts } => {
                violations.push(Violation {
                    path: path_str.clone(),
                    rule_id: CompactString::const_new("layout"),
                    message: CompactString::new(&reason),
                    severity,
                    fix_suggestion: None,
                    attempts,
                });
            }
            MatchResult::NotInLayout {
                nearest_valid,
                attempts,
            } => {
                let msg = if let Some(nearest) = nearest_valid {
                    format!("path not defined in layout (nearest valid: {})", nearest)
                } else {
                    "path not defined in layout".to_string()
                };
                violations.push(Violation {
                    path: path_str.clone(),
                    rule_id: CompactString::const_new("layout"),
                    message: CompactString::new(&msg),
                    severity,
                    fix_suggestion: None,
                    attempts,
                });
            }
            MatchResult::MissingRequired { expected } => {
                violations.push(Violation {
                    path: path_str,
                    rule_id: CompactString::const_new("layout"),
                    message: CompactString::new(format!(
                        "missing required children: {:?}",
                        expected
                    )),
                    severity,
                    fix_suggestion: None,
                    attempts: Vec::new(),
                });
            }
        }

        violations
    }

    pub fn explain_path(&self, path: &Path) -> PathExplanation {
        let match_result = self.layout_matcher.match_path(path);
        let expected = self.layout_matcher.get_expected_children(path);

        PathExplanation {
            path: path.to_path_buf(),
            match_result,
            expected_children: expected
                .into_iter()
                .map(|e| ExpectedChildInfo {
                    name: e.name,
                    is_dir: e.is_dir,
                    optional: e.optional,
                    is_param: e.is_param,
                })
                .collect(),
        }
    }
}

#[derive(Debug)]
pub struct PathExplanation {
    pub path: std::path::PathBuf,
    pub match_result: MatchResult,
    pub expected_children: Vec<ExpectedChildInfo>,
}

#[derive(Debug)]
pub struct ExpectedChildInfo {
    pub name: String,
    pub is_dir: bool,
    pub optional: bool,
    pub is_param: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CaseStyle, LayoutNode, Mode, RulesConfig};
    use std::collections::HashMap;

    fn create_test_config() -> ConfigIR {
        let mut module_children = HashMap::new();
        module_children.insert("index.ts".to_string(), LayoutNode::file());

        let mut services_children = HashMap::new();
        services_children.insert(
            "$module".to_string(),
            LayoutNode::param("module", CaseStyle::Kebab, LayoutNode::dir(module_children)),
        );

        let mut src_children = HashMap::new();
        src_children.insert("services".to_string(), LayoutNode::dir(services_children));

        let mut root_children = HashMap::new();
        root_children.insert("src".to_string(), LayoutNode::dir(src_children));

        ConfigIR {
            mode: Mode::Strict,
            layout: Some(LayoutNode::dir(root_children)),
            rules: RulesConfig {
                forbid_paths: vec!["**/utils/**".to_string()],
                forbid_names: vec!["temp".to_string()],
                ignore_paths: vec![],
            },
            boundaries: None,
            deps: None,
            ignore: vec![],
            use_gitignore: true,
            workspaces: vec![],
            dependencies: HashMap::new(),
            mirror: vec![],
            when: HashMap::new(),
            extends: None,
        }
    }

    #[test]
    fn test_check_valid_path() {
        let config = create_test_config();
        let matcher = FileMatcher::new(&config).unwrap();

        let violations = matcher.check_path(Path::new("src/services/billing/index.ts"));
        assert!(violations.is_empty());
    }

    #[test]
    fn test_check_forbidden_path() {
        let config = create_test_config();
        let matcher = FileMatcher::new(&config).unwrap();

        let violations = matcher.check_path(Path::new("src/utils/helper.ts"));
        assert!(!violations.is_empty());
        assert!(violations.iter().any(|v| v.rule_id == "forbidPaths"));
    }

    #[test]
    fn test_check_invalid_case() {
        let config = create_test_config();
        let matcher = FileMatcher::new(&config).unwrap();

        let violations = matcher.check_path(Path::new("src/services/MyModule/index.ts"));
        assert!(!violations.is_empty());
        assert!(violations.iter().any(|v| v.rule_id == "layout"));
    }
}
