use compact_str::CompactString;
use glob::Pattern;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::config::{ConfigIR, LayoutNode, MirrorConfig, WhenRequirement};

use super::matcher::{Severity, Violation};

pub struct PostValidator<'a> {
    config: &'a ConfigIR,
    all_paths: HashSet<PathBuf>,
    many_counts: HashMap<String, usize>,
}

impl<'a> PostValidator<'a> {
    pub fn new(config: &'a ConfigIR) -> Self {
        Self {
            config,
            all_paths: HashSet::new(),
            many_counts: HashMap::new(),
        }
    }

    pub fn record_path(&mut self, path: &Path) {
        self.all_paths.insert(path.to_path_buf());
    }

    pub fn record_many_match(&mut self, pattern_id: &str) {
        *self.many_counts.entry(pattern_id.to_string()).or_insert(0) += 1;
    }

    pub fn validate(&self, root_path: &Path, severity: Severity) -> Vec<Violation> {
        let mut violations = Vec::new();

        violations.extend(self.validate_required(root_path, severity));
        violations.extend(self.validate_dependencies(root_path, severity));
        violations.extend(self.validate_mirror(root_path, severity));
        violations.extend(self.validate_when(root_path, severity));

        violations
    }

    fn validate_required(&self, root_path: &Path, severity: Severity) -> Vec<Violation> {
        let mut violations = Vec::new();
        let mut required_paths = Vec::new();
        Self::collect_required_paths(&self.config.layout, PathBuf::new(), &mut required_paths);

        for req_path in required_paths {
            let full_path = root_path.join(&req_path);
            if !full_path.exists() {
                violations.push(Violation {
                    path: CompactString::new(req_path.to_string_lossy()),
                    rule_id: CompactString::const_new("required"),
                    message: CompactString::new(format!(
                        "required path does not exist: {}",
                        req_path.display()
                    )),
                    severity,
                    fix_suggestion: Some(CompactString::new(format!(
                        "create the required file or directory: {}",
                        req_path.display()
                    ))),
                    attempts: Vec::new(),
                });
            }
        }

        violations
    }

    fn collect_required_paths(
        node: &LayoutNode,
        current_path: PathBuf,
        paths: &mut Vec<PathBuf>,
    ) {
        match node {
            LayoutNode::Dir {
                children, required, ..
            } => {
                if *required && !current_path.as_os_str().is_empty() {
                    paths.push(current_path.clone());
                }
                for (name, child) in children {
                    if !name.starts_with('$') {
                        let child_path = current_path.join(name);
                        Self::collect_required_paths(child, child_path, paths);
                    }
                }
            }
            LayoutNode::File { required, .. } => {
                if *required && !current_path.as_os_str().is_empty() {
                    paths.push(current_path);
                }
            }
            _ => {}
        }
    }

    fn validate_dependencies(&self, root_path: &Path, severity: Severity) -> Vec<Violation> {
        let mut violations = Vec::new();

        for (source_pattern, target_pattern) in &self.config.dependencies {
            let source_glob = match Pattern::new(source_pattern) {
                Ok(p) => p,
                Err(_) => continue,
            };

            for source_path in &self.all_paths {
                let rel_path = source_path.strip_prefix(root_path).unwrap_or(source_path);
                let rel_str = rel_path.to_string_lossy();

                if source_glob.matches(&rel_str) {
                    let expected_target =
                        self.transform_glob_match(&rel_str, source_pattern, target_pattern);
                    let target_full = root_path.join(&expected_target);

                    if !target_full.exists() && !self.all_paths.contains(&target_full) {
                        violations.push(Violation {
                            path: CompactString::new(&rel_str),
                            rule_id: CompactString::const_new("dependency"),
                            message: CompactString::new(format!(
                                "missing required dependency: {} requires {}",
                                rel_str, expected_target
                            )),
                            severity,
                            fix_suggestion: Some(CompactString::new(format!(
                                "create: {}",
                                expected_target
                            ))),
                            attempts: Vec::new(),
                        });
                    }
                }
            }
        }

        violations
    }

    fn transform_glob_match(
        &self,
        path: &str,
        source_pattern: &str,
        target_pattern: &str,
    ) -> String {
        let source_parts: Vec<&str> = source_pattern.split('/').collect();
        let target_parts: Vec<&str> = target_pattern.split('/').collect();
        let path_parts: Vec<&str> = path.split('/').collect();

        let mut result_parts = Vec::new();

        for (i, target_part) in target_parts.iter().enumerate() {
            if *target_part == "*" || *target_part == "**" {
                if i < source_parts.len() && i < path_parts.len() {
                    if source_parts[i] == "*" || source_parts[i] == "**" {
                        result_parts.push(path_parts[i].to_string());
                    } else {
                        result_parts.push(target_part.to_string());
                    }
                } else {
                    result_parts.push(target_part.to_string());
                }
            } else if target_part.contains('*') {
                if i < path_parts.len() {
                    let filename = path_parts[path_parts.len() - 1];
                    let target_transformed = target_part.replace(
                        "*.test.ts",
                        &format!("{}.test.ts", filename.trim_end_matches(".ts")),
                    );
                    result_parts.push(target_transformed);
                } else {
                    result_parts.push(target_part.to_string());
                }
            } else {
                result_parts.push(target_part.to_string());
            }
        }

        result_parts.join("/")
    }

    fn validate_mirror(&self, root_path: &Path, severity: Severity) -> Vec<Violation> {
        let mut violations = Vec::new();

        for mirror in &self.config.mirror {
            violations.extend(self.check_mirror_rule(root_path, mirror, severity));
        }

        violations
    }

    fn check_mirror_rule(
        &self,
        root_path: &Path,
        mirror: &MirrorConfig,
        severity: Severity,
    ) -> Vec<Violation> {
        let mut violations = Vec::new();

        let source_glob = match Pattern::new(&mirror.source) {
            Ok(p) => p,
            Err(_) => return violations,
        };

        let (from_ext, to_ext) = self.parse_pattern_transform(&mirror.pattern);

        for source_path in &self.all_paths {
            let rel_path = source_path.strip_prefix(root_path).unwrap_or(source_path);
            let rel_str = rel_path.to_string_lossy();

            if source_glob.matches(&rel_str) {
                let expected_target = self.compute_mirror_target(
                    &rel_str,
                    &mirror.source,
                    &mirror.target,
                    &from_ext,
                    &to_ext,
                );
                let target_full = root_path.join(&expected_target);

                if !target_full.exists() && !self.all_paths.contains(&target_full) {
                    violations.push(Violation {
                        path: CompactString::new(&rel_str),
                        rule_id: CompactString::const_new("mirror"),
                        message: CompactString::new(format!(
                            "missing mirrored file: {} should have mirror at {}",
                            rel_str, expected_target
                        )),
                        severity,
                        fix_suggestion: Some(CompactString::new(format!(
                            "create mirrored file: {}",
                            expected_target
                        ))),
                        attempts: Vec::new(),
                    });
                }
            }
        }

        violations
    }

    fn parse_pattern_transform(&self, pattern: &str) -> (String, String) {
        let parts: Vec<&str> = pattern.split(" -> ").collect();
        if parts.len() == 2 {
            (parts[0].to_string(), parts[1].to_string())
        } else {
            ("*".to_string(), "*".to_string())
        }
    }

    fn compute_mirror_target(
        &self,
        source: &str,
        source_pattern: &str,
        target_pattern: &str,
        from_ext: &str,
        to_ext: &str,
    ) -> String {
        let source_base = source_pattern.replace('*', "");
        let target_base = target_pattern.replace('*', "");

        let mut target = source.replace(
            source_base.trim_matches('/'),
            target_base.trim_matches('/'),
        );

        if from_ext != "*" && to_ext != "*" {
            let from_suffix = from_ext.trim_start_matches('*');
            let to_suffix = to_ext.trim_start_matches('*');
            if target.ends_with(from_suffix) {
                target = format!(
                    "{}{}",
                    &target[..target.len() - from_suffix.len()],
                    to_suffix
                );
            }
        }

        target
    }

    fn validate_when(&self, root_path: &Path, severity: Severity) -> Vec<Violation> {
        let mut violations = Vec::new();

        for (trigger_file, requirement) in &self.config.when {
            violations.extend(self.check_when_rule(root_path, trigger_file, requirement, severity));
        }

        violations
    }

    fn check_when_rule(
        &self,
        root_path: &Path,
        trigger_file: &str,
        requirement: &WhenRequirement,
        severity: Severity,
    ) -> Vec<Violation> {
        let mut violations = Vec::new();

        for path in &self.all_paths {
            let rel_path = path.strip_prefix(root_path).unwrap_or(path);
            let filename = rel_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string());

            if filename.as_deref() == Some(trigger_file) {
                let parent = rel_path.parent().unwrap_or(Path::new(""));

                for required_file in &requirement.requires {
                    let required_path = parent.join(required_file);
                    let full_required = root_path.join(&required_path);

                    if !full_required.exists() {
                        let check_in_paths = self.all_paths.iter().any(|p| {
                            p.strip_prefix(root_path)
                                .map(|rp| rp == required_path)
                                .unwrap_or(false)
                        });

                        if !check_in_paths {
                            violations.push(Violation {
                                path: CompactString::new(rel_path.to_string_lossy()),
                                rule_id: CompactString::const_new("when"),
                                message: CompactString::new(format!(
                                    "{} requires {} to exist in the same directory",
                                    trigger_file, required_file
                                )),
                                severity,
                                fix_suggestion: Some(CompactString::new(format!(
                                    "create: {}",
                                    required_path.display()
                                ))),
                                attempts: Vec::new(),
                            });
                        }
                    }
                }
            }
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Mode, RulesConfig};
    use std::collections::HashMap;

    fn create_test_config() -> ConfigIR {
        ConfigIR {
            mode: Mode::Strict,
            layout: LayoutNode::dir(HashMap::new()),
            rules: RulesConfig::default(),
            boundaries: None,
            deps: None,
            ignore: vec![],
            use_gitignore: true,
            workspaces: vec![],
            dependencies: HashMap::new(),
            mirror: vec![],
            when: HashMap::new(),
        }
    }

    #[test]
    fn test_empty_validation() {
        let config = create_test_config();
        let validator = PostValidator::new(&config);
        let violations = validator.validate(Path::new("."), Severity::Error);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_required_path_collection() {
        let mut children = HashMap::new();
        children.insert(
            "required.ts".to_string(),
            LayoutNode::File {
                pattern: None,
                optional: false,
                required: true,
                case: None,
            },
        );
        children.insert(
            "optional.ts".to_string(),
            LayoutNode::File {
                pattern: None,
                optional: true,
                required: false,
                case: None,
            },
        );

        let config = ConfigIR {
            mode: Mode::Strict,
            layout: LayoutNode::Dir {
                children,
                optional: false,
                required: false,
                strict: false,
                max_depth: None,
            },
            rules: RulesConfig::default(),
            boundaries: None,
            deps: None,
            ignore: vec![],
            use_gitignore: true,
            workspaces: vec![],
            dependencies: HashMap::new(),
            mirror: vec![],
            when: HashMap::new(),
        };

        let _validator = PostValidator::new(&config);
        let mut paths = Vec::new();
        PostValidator::collect_required_paths(&config.layout, PathBuf::new(), &mut paths);

        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], PathBuf::from("required.ts"));
    }

    #[test]
    fn test_when_validation() {
        let mut when = HashMap::new();
        when.insert(
            "controller.ts".to_string(),
            WhenRequirement {
                requires: vec!["service.ts".to_string()],
            },
        );

        let config = ConfigIR {
            mode: Mode::Strict,
            layout: LayoutNode::dir(HashMap::new()),
            rules: RulesConfig::default(),
            boundaries: None,
            deps: None,
            ignore: vec![],
            use_gitignore: true,
            workspaces: vec![],
            dependencies: HashMap::new(),
            mirror: vec![],
            when,
        };

        let mut validator = PostValidator::new(&config);
        validator.record_path(Path::new("/test/src/controller.ts"));

        let violations = validator.validate(Path::new("/test"), Severity::Error);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule_id.as_str(), "when");
    }
}
