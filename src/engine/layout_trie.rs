use std::path::Path;

use crate::config::LayoutNode;

#[derive(Debug, Clone)]
pub struct MatchAttempt {
    pub pattern: String,
    pub matched: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub enum MatchResult {
    Allowed,
    AllowedParam { name: String, value: String },
    AllowedMany { values: Vec<String> },
    Denied { reason: String, attempts: Vec<MatchAttempt> },
    NotInLayout { nearest_valid: Option<String>, attempts: Vec<MatchAttempt> },
    MissingRequired { expected: Vec<String> },
}

#[derive(Debug, Clone)]
pub struct LayoutMatcher {
    root: LayoutNode,
}

impl LayoutMatcher {
    pub fn new(root: LayoutNode) -> Self {
        Self { root }
    }

    pub fn match_path(&self, path: &Path) -> MatchResult {
        let components: Vec<&str> = path
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect();

        if components.is_empty() {
            return MatchResult::Allowed;
        }

        self.match_segments(&components, &self.root, Vec::new())
    }

    fn match_segments(
        &self,
        segments: &[&str],
        node: &LayoutNode,
        path_so_far: Vec<String>,
    ) -> MatchResult {
        if segments.is_empty() {
            return match node {
                LayoutNode::Dir { .. } => MatchResult::Allowed,
                LayoutNode::File { .. } => MatchResult::Allowed,
                _ => MatchResult::Allowed,
            };
        }

        let current = segments[0];
        let remaining = &segments[1..];

        match node {
            LayoutNode::Dir { children, .. } => {
                if let Some(child) = children.get(current) {
                    return self.match_segments(
                        remaining,
                        child,
                        self.extend_path(&path_so_far, current),
                    );
                }

                for (key, child) in children {
                    if key.starts_with('$') {
                        match child {
                            LayoutNode::Param {
                                name,
                                case,
                                child: inner,
                            } => {
                                if case.validate(current) {
                                    let result = self.match_segments(
                                        remaining,
                                        inner,
                                        self.extend_path(&path_so_far, current),
                                    );
                                    if matches!(
                                        result,
                                        MatchResult::Allowed
                                            | MatchResult::AllowedParam { .. }
                                            | MatchResult::AllowedMany { .. }
                                    ) {
                                        return MatchResult::AllowedParam {
                                            name: name.clone(),
                                            value: current.to_string(),
                                        };
                                    }
                                } else {
                                    return MatchResult::Denied {
                                        reason: format!(
                                            "'{}' does not match {} case for parameter {}",
                                            current,
                                            format!("{:?}", case).to_lowercase(),
                                            name
                                        ),
                                        attempts: vec![MatchAttempt {
                                            pattern: format!("${}", name),
                                            matched: false,
                                            reason: Some(format!(
                                                "expected {} case",
                                                format!("{:?}", case).to_lowercase()
                                            )),
                                        }],
                                    };
                                }
                            }
                            LayoutNode::Many { case, child: inner } => {
                                if let Some(case_style) = case {
                                    if !case_style.validate(current) {
                                        return MatchResult::Denied {
                                            reason: format!(
                                                "'{}' does not match {} case",
                                                current,
                                                format!("{:?}", case_style).to_lowercase()
                                            ),
                                            attempts: vec![MatchAttempt {
                                                pattern: format!("$many({})", format!("{:?}", case_style).to_lowercase()),
                                                matched: false,
                                                reason: Some(format!(
                                                    "expected {} case",
                                                    format!("{:?}", case_style).to_lowercase()
                                                )),
                                            }],
                                        };
                                    }
                                }
                                let result = self.match_segments(
                                    remaining,
                                    inner,
                                    self.extend_path(&path_so_far, current),
                                );
                                if matches!(
                                    result,
                                    MatchResult::Allowed
                                        | MatchResult::AllowedParam { .. }
                                        | MatchResult::AllowedMany { .. }
                                ) {
                                    return MatchResult::AllowedMany {
                                        values: vec![current.to_string()],
                                    };
                                }
                                return result;
                            }
                            LayoutNode::Recursive {
                                max_depth,
                                child: inner,
                            } => {
                                let result = self.match_recursive(
                                    segments,
                                    inner,
                                    path_so_far.clone(),
                                    *max_depth,
                                    0,
                                );
                                if matches!(
                                    result,
                                    MatchResult::Allowed
                                        | MatchResult::AllowedParam { .. }
                                        | MatchResult::AllowedMany { .. }
                                ) {
                                    return result;
                                }
                            }
                            LayoutNode::Either { variants } => {
                                for variant in variants {
                                    let result = self.match_segments(
                                        segments,
                                        variant,
                                        path_so_far.clone(),
                                    );
                                    if matches!(
                                        result,
                                        MatchResult::Allowed
                                            | MatchResult::AllowedParam { .. }
                                            | MatchResult::AllowedMany { .. }
                                    ) {
                                        return result;
                                    }
                                }
                            }
                            _ => continue,
                        }
                    }
                }

                let expected: Vec<String> = children
                    .keys()
                    .filter(|k| !k.starts_with('$'))
                    .take(5)
                    .cloned()
                    .collect();

                let param_keys: Vec<String> = children
                    .keys()
                    .filter(|k| k.starts_with('$'))
                    .cloned()
                    .collect();

                let mut attempts = Vec::new();
                for key in &expected {
                    attempts.push(MatchAttempt {
                        pattern: key.clone(),
                        matched: false,
                        reason: Some(format!("expected literal '{}'", key)),
                    });
                }
                for key in &param_keys {
                    attempts.push(MatchAttempt {
                        pattern: key.clone(),
                        matched: false,
                        reason: Some("parameter/pattern did not match".to_string()),
                    });
                }

                MatchResult::NotInLayout {
                    nearest_valid: if path_so_far.is_empty() {
                        None
                    } else {
                        Some(path_so_far.join("/"))
                    },
                    attempts,
                }
            }
            LayoutNode::File { pattern, .. } => {
                if !remaining.is_empty() {
                    return MatchResult::Denied {
                        reason: format!("'{}' is a file, cannot have children", current),
                        attempts: vec![MatchAttempt {
                            pattern: "file".to_string(),
                            matched: false,
                            reason: Some("files cannot have child paths".to_string()),
                        }],
                    };
                }

                if let Some(pat) = pattern {
                    if !Self::matches_pattern(current, pat) {
                        return MatchResult::Denied {
                            reason: format!("'{}' does not match pattern '{}'", current, pat),
                            attempts: vec![MatchAttempt {
                                pattern: pat.clone(),
                                matched: false,
                                reason: Some(format!("filename must match pattern '{}'", pat)),
                            }],
                        };
                    }
                }

                MatchResult::Allowed
            }
            LayoutNode::Param { name, case, child } => {
                if case.validate(current) {
                    let result = self.match_segments(
                        remaining,
                        child,
                        self.extend_path(&path_so_far, current),
                    );
                    if matches!(
                        result,
                        MatchResult::Allowed
                            | MatchResult::AllowedParam { .. }
                            | MatchResult::AllowedMany { .. }
                    ) {
                        return MatchResult::AllowedParam {
                            name: name.clone(),
                            value: current.to_string(),
                        };
                    }
                    return result;
                }
                MatchResult::Denied {
                    reason: format!(
                        "'{}' does not match {} case for parameter {}",
                        current,
                        format!("{:?}", case).to_lowercase(),
                        name
                    ),
                    attempts: vec![MatchAttempt {
                        pattern: format!("${}", name),
                        matched: false,
                        reason: Some(format!(
                            "expected {} case",
                            format!("{:?}", case).to_lowercase()
                        )),
                    }],
                }
            }
            LayoutNode::Many { case, child } => {
                if let Some(case_style) = case {
                    if !case_style.validate(current) {
                        return MatchResult::Denied {
                            reason: format!(
                                "'{}' does not match {} case",
                                current,
                                format!("{:?}", case_style).to_lowercase()
                            ),
                            attempts: vec![MatchAttempt {
                                pattern: format!("$many({})", format!("{:?}", case_style).to_lowercase()),
                                matched: false,
                                reason: Some(format!(
                                    "expected {} case",
                                    format!("{:?}", case_style).to_lowercase()
                                )),
                            }],
                        };
                    }
                }
                self.match_segments(remaining, child, self.extend_path(&path_so_far, current))
            }
            LayoutNode::Recursive { max_depth, child } => {
                self.match_recursive(
                    segments,
                    child,
                    path_so_far,
                    *max_depth,
                    0,
                )
            }
            LayoutNode::Either { variants } => {
                let mut all_attempts = Vec::new();
                for (i, variant) in variants.iter().enumerate() {
                    let result = self.match_segments(
                        segments,
                        variant,
                        path_so_far.clone(),
                    );
                    match &result {
                        MatchResult::Allowed
                        | MatchResult::AllowedParam { .. }
                        | MatchResult::AllowedMany { .. } => return result,
                        MatchResult::Denied { attempts, .. }
                        | MatchResult::NotInLayout { attempts, .. } => {
                            all_attempts.push(MatchAttempt {
                                pattern: format!("either[{}]", i),
                                matched: false,
                                reason: attempts.first().and_then(|a| a.reason.clone()),
                            });
                        }
                        _ => {
                            all_attempts.push(MatchAttempt {
                                pattern: format!("either[{}]", i),
                                matched: false,
                                reason: None,
                            });
                        }
                    }
                }
                MatchResult::NotInLayout {
                    nearest_valid: if path_so_far.is_empty() {
                        None
                    } else {
                        Some(path_so_far.join("/"))
                    },
                    attempts: all_attempts,
                }
            }
        }
    }

    fn match_recursive(
        &self,
        segments: &[&str],
        child: &LayoutNode,
        path_so_far: Vec<String>,
        max_depth: usize,
        current_depth: usize,
    ) -> MatchResult {
        if segments.is_empty() {
            return MatchResult::Allowed;
        }

        if current_depth >= max_depth {
            return MatchResult::NotInLayout {
                nearest_valid: if path_so_far.is_empty() {
                    None
                } else {
                    Some(path_so_far.join("/"))
                },
                attempts: vec![MatchAttempt {
                    pattern: "recursive".to_string(),
                    matched: false,
                    reason: Some(format!("exceeded max depth of {}", max_depth)),
                }],
            };
        }

        let result = self.match_segments(segments, child, path_so_far.clone());
        if matches!(
            result,
            MatchResult::Allowed | MatchResult::AllowedParam { .. } | MatchResult::AllowedMany { .. }
        ) {
            return result;
        }

        let current = segments[0];
        let remaining = &segments[1..];
        let new_path = self.extend_path(&path_so_far, current);

        self.match_recursive(remaining, child, new_path, max_depth, current_depth + 1)
    }

    fn extend_path(&self, path: &[String], segment: &str) -> Vec<String> {
        let mut new_path = path.to_vec();
        new_path.push(segment.to_string());
        new_path
    }

    fn matches_pattern(name: &str, pattern: &str) -> bool {
        if pattern.contains('*') {
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                let prefix = parts[0];
                let suffix = parts[1];
                return name.starts_with(prefix) && name.ends_with(suffix);
            }
        }
        name == pattern
    }

    pub fn get_expected_children(&self, path: &Path) -> Vec<ExpectedChild> {
        let components: Vec<&str> = path
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect();

        self.get_expected_at(&components, &self.root)
    }

    fn get_expected_at(&self, segments: &[&str], node: &LayoutNode) -> Vec<ExpectedChild> {
        if segments.is_empty() {
            return self.collect_expected(node);
        }

        let current = segments[0];
        let remaining = &segments[1..];

        match node {
            LayoutNode::Dir { children, .. } => {
                if let Some(child) = children.get(current) {
                    return self.get_expected_at(remaining, child);
                }
                for (key, child) in children {
                    if key.starts_with('$') {
                        return self.get_expected_at(remaining, child);
                    }
                }
                Vec::new()
            }
            LayoutNode::Param { child, .. } => self.get_expected_at(remaining, child),
            LayoutNode::Many { child, .. } => self.get_expected_at(remaining, child),
            LayoutNode::Recursive { child, .. } => self.get_expected_at(remaining, child),
            LayoutNode::Either { variants } => {
                for variant in variants {
                    let result = self.get_expected_at(remaining, variant);
                    if !result.is_empty() {
                        return result;
                    }
                }
                Vec::new()
            }
            LayoutNode::File { .. } => Vec::new(),
        }
    }

    fn collect_expected(&self, node: &LayoutNode) -> Vec<ExpectedChild> {
        match node {
            LayoutNode::Dir { children, .. } => children
                .iter()
                .map(|(name, child)| ExpectedChild {
                    name: name.clone(),
                    is_dir: matches!(
                        child,
                        LayoutNode::Dir { .. }
                            | LayoutNode::Param { .. }
                            | LayoutNode::Many { .. }
                            | LayoutNode::Recursive { .. }
                    ),
                    optional: child.is_optional(),
                    is_param: name.starts_with('$'),
                })
                .collect(),
            LayoutNode::Recursive { child, .. } => self.collect_expected(child),
            LayoutNode::Either { variants } => {
                variants
                    .iter()
                    .flat_map(|v| self.collect_expected(v))
                    .collect()
            }
            _ => Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExpectedChild {
    pub name: String,
    pub is_dir: bool,
    pub optional: bool,
    pub is_param: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CaseStyle;
    use std::collections::HashMap;

    fn create_simple_layout() -> LayoutNode {
        let mut src_children = HashMap::new();
        src_children.insert("index.ts".to_string(), LayoutNode::file());

        let mut root_children = HashMap::new();
        root_children.insert("src".to_string(), LayoutNode::dir(src_children));

        LayoutNode::dir(root_children)
    }

    #[test]
    fn test_match_valid_path() {
        let layout = create_simple_layout();
        let matcher = LayoutMatcher::new(layout);

        let result = matcher.match_path(Path::new("src"));
        assert!(matches!(result, MatchResult::Allowed));

        let result = matcher.match_path(Path::new("src/index.ts"));
        assert!(matches!(result, MatchResult::Allowed));
    }

    #[test]
    fn test_match_invalid_path() {
        let layout = create_simple_layout();
        let matcher = LayoutMatcher::new(layout);

        let result = matcher.match_path(Path::new("lib"));
        assert!(matches!(result, MatchResult::NotInLayout { .. }));
    }

    #[test]
    fn test_match_param() {
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

        let layout = LayoutNode::dir(root_children);
        let matcher = LayoutMatcher::new(layout);

        let result = matcher.match_path(Path::new("src/services/my-module"));
        assert!(matches!(result, MatchResult::AllowedParam { .. }));

        let result = matcher.match_path(Path::new("src/services/MyModule"));
        assert!(matches!(result, MatchResult::Denied { .. }));
    }

    #[test]
    fn test_case_validation_in_match() {
        let mut module_children = HashMap::new();
        module_children.insert("index.ts".to_string(), LayoutNode::file());

        let mut services_children = HashMap::new();
        services_children.insert(
            "$module".to_string(),
            LayoutNode::param("module", CaseStyle::Snake, LayoutNode::dir(module_children)),
        );

        let mut src_children = HashMap::new();
        src_children.insert("services".to_string(), LayoutNode::dir(services_children));

        let mut root_children = HashMap::new();
        root_children.insert("src".to_string(), LayoutNode::dir(src_children));

        let layout = LayoutNode::dir(root_children);
        let matcher = LayoutMatcher::new(layout);

        let result = matcher.match_path(Path::new("src/services/my_module/index.ts"));
        assert!(matches!(result, MatchResult::AllowedParam { .. }));
    }

    #[test]
    fn test_recursive_matching() {
        let mut route_children = HashMap::new();
        route_children.insert("page.tsx".to_string(), LayoutNode::file());

        let recursive = LayoutNode::recursive(LayoutNode::param(
            "route",
            CaseStyle::Kebab,
            LayoutNode::dir(route_children),
        ));

        let mut app_children = HashMap::new();
        app_children.insert("$routes".to_string(), recursive);

        let mut root_children = HashMap::new();
        root_children.insert("app".to_string(), LayoutNode::dir(app_children));

        let layout = LayoutNode::dir(root_children);
        let matcher = LayoutMatcher::new(layout);

        let result = matcher.match_path(Path::new("app/dashboard/page.tsx"));
        assert!(matches!(result, MatchResult::Allowed | MatchResult::AllowedParam { .. }));

        let result = matcher.match_path(Path::new("app/dashboard/settings/page.tsx"));
        assert!(matches!(result, MatchResult::Allowed | MatchResult::AllowedParam { .. }));

        let result = matcher.match_path(Path::new("app/dashboard/settings/profile/page.tsx"));
        assert!(matches!(result, MatchResult::Allowed | MatchResult::AllowedParam { .. }));
    }

    #[test]
    fn test_recursive_with_depth_limit() {
        let mut route_children = HashMap::new();
        route_children.insert("page.tsx".to_string(), LayoutNode::file());

        let recursive = LayoutNode::recursive_with_depth(
            2,
            LayoutNode::param("route", CaseStyle::Kebab, LayoutNode::dir(route_children)),
        );

        let mut app_children = HashMap::new();
        app_children.insert("$routes".to_string(), recursive);

        let mut root_children = HashMap::new();
        root_children.insert("app".to_string(), LayoutNode::dir(app_children));

        let layout = LayoutNode::dir(root_children);
        let matcher = LayoutMatcher::new(layout);

        let result = matcher.match_path(Path::new("app/a/page.tsx"));
        assert!(matches!(result, MatchResult::Allowed | MatchResult::AllowedParam { .. }));

        let result = matcher.match_path(Path::new("app/a/b/page.tsx"));
        assert!(matches!(result, MatchResult::Allowed | MatchResult::AllowedParam { .. }));

        let result = matcher.match_path(Path::new("app/a/b/c/page.tsx"));
        assert!(matches!(result, MatchResult::NotInLayout { .. }));
    }

    #[test]
    fn test_either_matching() {
        let file_variant = LayoutNode::file_with_pattern("page.tsx");
        let mut dir_children = HashMap::new();
        dir_children.insert("index.ts".to_string(), LayoutNode::file());
        let dir_variant = LayoutNode::dir(dir_children);

        let either = LayoutNode::either(vec![file_variant, dir_variant]);

        let mut routes_children = HashMap::new();
        routes_children.insert("$segment".to_string(), LayoutNode::param("segment", CaseStyle::Kebab, either));

        let mut root_children = HashMap::new();
        root_children.insert("routes".to_string(), LayoutNode::dir(routes_children));

        let layout = LayoutNode::dir(root_children);
        let matcher = LayoutMatcher::new(layout);

        let result = matcher.match_path(Path::new("routes/dashboard/index.ts"));
        assert!(matches!(result, MatchResult::Allowed | MatchResult::AllowedParam { .. }));
    }

    #[test]
    fn test_match_attempts_in_denied_result() {
        let mut module_children = HashMap::new();
        module_children.insert("index.ts".to_string(), LayoutNode::file());

        let mut services_children = HashMap::new();
        services_children.insert(
            "$module".to_string(),
            LayoutNode::param("module", CaseStyle::Kebab, LayoutNode::dir(module_children)),
        );

        let mut root_children = HashMap::new();
        root_children.insert("services".to_string(), LayoutNode::dir(services_children));

        let layout = LayoutNode::dir(root_children);
        let matcher = LayoutMatcher::new(layout);

        let result = matcher.match_path(Path::new("services/MyModule/index.ts"));
        match result {
            MatchResult::Denied { reason, attempts } => {
                assert!(reason.contains("does not match"));
                assert!(!attempts.is_empty());
                assert!(attempts.iter().any(|a| a.pattern.starts_with('$')));
            }
            _ => panic!("Expected Denied result"),
        }
    }

    #[test]
    fn test_match_attempts_in_not_in_layout() {
        let mut root_children = HashMap::new();
        root_children.insert("src".to_string(), LayoutNode::dir(HashMap::new()));
        root_children.insert("lib".to_string(), LayoutNode::dir(HashMap::new()));

        let layout = LayoutNode::dir(root_children);
        let matcher = LayoutMatcher::new(layout);

        let result = matcher.match_path(Path::new("unknown/file.ts"));
        match result {
            MatchResult::NotInLayout { attempts, .. } => {
                assert!(!attempts.is_empty());
                assert!(attempts.iter().any(|a| a.pattern == "src" || a.pattern == "lib"));
            }
            _ => panic!("Expected NotInLayout result"),
        }
    }

    #[test]
    fn test_recursive_depth_exceeded_returns_not_in_layout() {
        let mut route_children = HashMap::new();
        route_children.insert("page.tsx".to_string(), LayoutNode::file());

        let recursive = LayoutNode::recursive_with_depth(
            2,
            LayoutNode::param("route", CaseStyle::Kebab, LayoutNode::dir(route_children)),
        );

        let mut app_children = HashMap::new();
        app_children.insert("$routes".to_string(), recursive);

        let mut root_children = HashMap::new();
        root_children.insert("app".to_string(), LayoutNode::dir(app_children));

        let layout = LayoutNode::dir(root_children);
        let matcher = LayoutMatcher::new(layout);

        let result = matcher.match_path(Path::new("app/a/b/c/page.tsx"));
        assert!(matches!(result, MatchResult::NotInLayout { .. }), "Expected NotInLayout result, got {:?}", result);
    }

    #[test]
    fn test_either_all_variants_fail_has_attempts() {
        let file_variant = LayoutNode::file_with_pattern("*.tsx");
        let mut dir_children = HashMap::new();
        dir_children.insert("index.ts".to_string(), LayoutNode::file());
        let dir_variant = LayoutNode::dir(dir_children);

        let either = LayoutNode::either(vec![file_variant, dir_variant]);

        let mut root_children = HashMap::new();
        root_children.insert("routes".to_string(), either);

        let layout = LayoutNode::dir(root_children);
        let matcher = LayoutMatcher::new(layout);

        let result = matcher.match_path(Path::new("routes/unknown.js"));
        match result {
            MatchResult::NotInLayout { attempts, .. } | MatchResult::Denied { attempts, .. } => {
                assert!(attempts.iter().any(|a| a.pattern.starts_with("either")));
            }
            _ => panic!("Expected NotInLayout or Denied result"),
        }
    }

    #[test]
    fn test_deeply_nested_recursive_with_multiple_files() {
        let mut route_children = HashMap::new();
        route_children.insert("page.tsx".to_string(), LayoutNode::file());
        route_children.insert("layout.tsx".to_string(), LayoutNode::file().optional());
        route_children.insert("loading.tsx".to_string(), LayoutNode::file().optional());

        let recursive = LayoutNode::recursive(LayoutNode::param(
            "route",
            CaseStyle::Kebab,
            LayoutNode::dir(route_children),
        ));

        let mut app_children = HashMap::new();
        app_children.insert("$routes".to_string(), recursive);

        let mut root_children = HashMap::new();
        root_children.insert("app".to_string(), LayoutNode::dir(app_children));

        let layout = LayoutNode::dir(root_children);
        let matcher = LayoutMatcher::new(layout);

        let result = matcher.match_path(Path::new("app/dashboard/settings/profile/page.tsx"));
        assert!(matches!(result, MatchResult::Allowed | MatchResult::AllowedParam { .. }));

        let result = matcher.match_path(Path::new("app/dashboard/settings/profile/layout.tsx"));
        assert!(matches!(result, MatchResult::Allowed | MatchResult::AllowedParam { .. }));

        let result = matcher.match_path(Path::new("app/a/b/c/d/e/f/g/page.tsx"));
        assert!(matches!(result, MatchResult::Allowed | MatchResult::AllowedParam { .. }));
    }
}
