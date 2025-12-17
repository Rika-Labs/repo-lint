use std::path::Path;

use crate::config::LayoutNode;

#[derive(Debug, Clone)]
pub enum MatchResult {
    Allowed,
    AllowedParam { name: String, value: String },
    AllowedMany { values: Vec<String> },
    Denied { reason: String },
    NotInLayout { nearest_valid: Option<String> },
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
                            _ => continue,
                        }
                    }
                }

                let _expected: Vec<String> = children
                    .keys()
                    .filter(|k| !k.starts_with('$'))
                    .take(5)
                    .cloned()
                    .collect();

                MatchResult::NotInLayout {
                    nearest_valid: if path_so_far.is_empty() {
                        None
                    } else {
                        Some(path_so_far.join("/"))
                    },
                }
            }
            LayoutNode::File { pattern, .. } => {
                if !remaining.is_empty() {
                    return MatchResult::Denied {
                        reason: format!("'{}' is a file, cannot have children", current),
                    };
                }

                if let Some(pat) = pattern {
                    if !Self::matches_pattern(current, pat) {
                        return MatchResult::Denied {
                            reason: format!("'{}' does not match pattern '{}'", current, pat),
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
                        };
                    }
                }
                self.match_segments(remaining, child, self.extend_path(&path_so_far, current))
            }
        }
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
                        LayoutNode::Dir { .. } | LayoutNode::Param { .. } | LayoutNode::Many { .. }
                    ),
                    optional: child.is_optional(),
                    is_param: name.starts_with('$'),
                })
                .collect(),
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
}
