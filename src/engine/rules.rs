use compact_str::CompactString;
use std::path::Path;

use crate::config::RulesConfig;

#[derive(Debug, Clone)]
pub struct CompiledRules {
    forbid_paths_patterns: Vec<CompactString>,
    forbid_names: Vec<CompactString>,
    forbid_names_lower: Vec<CompactString>,
    has_path_rules: bool,
    has_name_rules: bool,
}

impl CompiledRules {
    pub fn compile(config: &RulesConfig) -> Result<Self, globset::Error> {
        let forbid_paths_patterns: Vec<CompactString> = config
            .forbid_paths
            .iter()
            .map(|s| CompactString::new(s))
            .collect();

        let forbid_names: Vec<CompactString> = config
            .forbid_names
            .iter()
            .map(|s| CompactString::new(s))
            .collect();

        let forbid_names_lower: Vec<CompactString> = config
            .forbid_names
            .iter()
            .map(|n| CompactString::new(n.to_lowercase()))
            .collect();

        Ok(Self {
            has_path_rules: !config.forbid_paths.is_empty(),
            has_name_rules: !config.forbid_names.is_empty(),
            forbid_paths_patterns,
            forbid_names,
            forbid_names_lower,
        })
    }

    #[inline]
    pub fn has_rules(&self) -> bool {
        self.has_path_rules || self.has_name_rules
    }

    #[inline]
    pub fn check_path(&self, path: &Path) -> Vec<RuleViolation> {
        if !self.has_path_rules && !self.has_name_rules {
            return Vec::new();
        }

        let mut violations = Vec::with_capacity(2);
        let path_str = path.to_string_lossy();

        if self.has_path_rules {
            let mut matching_patterns = Vec::new();
            for pattern in &self.forbid_paths_patterns {
                if fast_glob::glob_match(pattern.as_str(), path_str.as_ref()) {
                    matching_patterns.push(pattern.to_string());
                }
            }
            if !matching_patterns.is_empty() {
                violations.push(RuleViolation::ForbiddenPath {
                    path: path.to_path_buf(),
                    matched_patterns: matching_patterns,
                });
            }
        }

        if self.has_name_rules {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                let name_bytes = name.as_bytes();
                let name_len = name_bytes.len();

                let mut name_lower_buf = [0u8; 256];
                let name_lower = if name_len <= 256 {
                    for (i, &b) in name_bytes.iter().enumerate() {
                        name_lower_buf[i] = b.to_ascii_lowercase();
                    }
                    unsafe { std::str::from_utf8_unchecked(&name_lower_buf[..name_len]) }
                } else {
                    return violations;
                };

                let stem_end = name_lower
                    .rfind('.')
                    .filter(|&pos| pos > 0)
                    .unwrap_or(name_len);
                let stem_lower = &name_lower[..stem_end];

                for (idx, forbidden_lower) in self.forbid_names_lower.iter().enumerate() {
                    let forbidden_bytes = forbidden_lower.as_bytes();
                    let forbidden_len = forbidden_bytes.len();

                    let stem_match =
                        stem_end == forbidden_len && stem_lower.as_bytes() == forbidden_bytes;

                    let full_match =
                        name_len == forbidden_len && name_lower.as_bytes() == forbidden_bytes;

                    if stem_match || full_match {
                        violations.push(RuleViolation::ForbiddenName {
                            path: path.to_path_buf(),
                            name: name.to_string(),
                            forbidden: self.forbid_names[idx].to_string(),
                        });
                        break;
                    }
                }
            }
        }

        violations
    }

    pub fn check_directory_name(&self, name: &str, path: &Path) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        let name_lower = name.to_lowercase();

        for (idx, forbidden_lower) in self.forbid_names_lower.iter().enumerate() {
            if name_lower == forbidden_lower.as_str() {
                violations.push(RuleViolation::ForbiddenName {
                    path: path.to_path_buf(),
                    name: name.to_string(),
                    forbidden: self.forbid_names[idx].to_string(),
                });
            }
        }

        violations
    }
}

#[derive(Debug, Clone)]
pub enum RuleViolation {
    ForbiddenPath {
        path: std::path::PathBuf,
        matched_patterns: Vec<String>,
    },
    ForbiddenName {
        path: std::path::PathBuf,
        name: String,
        forbidden: String,
    },
}

impl RuleViolation {
    pub fn path(&self) -> &Path {
        match self {
            RuleViolation::ForbiddenPath { path, .. } => path,
            RuleViolation::ForbiddenName { path, .. } => path,
        }
    }

    pub fn rule_id(&self) -> &'static str {
        match self {
            RuleViolation::ForbiddenPath { .. } => "forbidPaths",
            RuleViolation::ForbiddenName { .. } => "forbidNames",
        }
    }

    pub fn message(&self) -> String {
        match self {
            RuleViolation::ForbiddenPath {
                matched_patterns, ..
            } => {
                format!(
                    "path matches forbidden pattern: {}",
                    matched_patterns.join(", ")
                )
            }
            RuleViolation::ForbiddenName {
                name, forbidden, ..
            } => {
                format!("'{}' matches forbidden name '{}'", name, forbidden)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forbid_paths() {
        let config = RulesConfig {
            forbid_paths: vec!["**/utils/**".to_string(), "**/*.bak".to_string()],
            forbid_names: vec![],
        };

        let rules = CompiledRules::compile(&config).unwrap();

        let violations = rules.check_path(Path::new("src/utils/helper.ts"));
        assert_eq!(violations.len(), 1);

        let violations = rules.check_path(Path::new("backup.bak"));
        assert_eq!(violations.len(), 1);

        let violations = rules.check_path(Path::new("src/services/billing.ts"));
        assert!(violations.is_empty());
    }

    #[test]
    fn test_forbid_names() {
        let config = RulesConfig {
            forbid_paths: vec![],
            forbid_names: vec!["temp".to_string(), "test".to_string(), "new".to_string()],
        };

        let rules = CompiledRules::compile(&config).unwrap();

        let violations = rules.check_path(Path::new("src/temp.ts"));
        assert_eq!(violations.len(), 1);

        let violations = rules.check_path(Path::new("src/TEMP.ts"));
        assert_eq!(violations.len(), 1);

        let violations = rules.check_path(Path::new("src/new.ts"));
        assert_eq!(violations.len(), 1);

        let violations = rules.check_path(Path::new("src/billing.ts"));
        assert!(violations.is_empty());
    }

    #[test]
    fn test_combined_rules() {
        let config = RulesConfig {
            forbid_paths: vec!["**/utils/**".to_string()],
            forbid_names: vec!["temp".to_string()],
        };

        let rules = CompiledRules::compile(&config).unwrap();

        let violations = rules.check_path(Path::new("src/utils/temp.ts"));
        assert_eq!(violations.len(), 2);
    }
}
