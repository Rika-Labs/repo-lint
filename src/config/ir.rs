use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Strict,
    Warn,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CaseStyle {
    Kebab,
    Snake,
    Camel,
    Pascal,
    Any,
}

impl CaseStyle {
    pub fn validate(&self, s: &str) -> bool {
        match self {
            CaseStyle::Kebab => Self::is_kebab_case(s),
            CaseStyle::Snake => Self::is_snake_case(s),
            CaseStyle::Camel => Self::is_camel_case(s),
            CaseStyle::Pascal => Self::is_pascal_case(s),
            CaseStyle::Any => true,
        }
    }

    fn is_kebab_case(s: &str) -> bool {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            && !s.starts_with('-')
            && !s.ends_with('-')
            && !s.contains("--")
    }

    fn is_snake_case(s: &str) -> bool {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            && !s.starts_with('_')
            && !s.ends_with('_')
            && !s.contains("__")
    }

    fn is_camel_case(s: &str) -> bool {
        if s.is_empty() {
            return false;
        }
        let mut chars = s.chars();
        let first = chars.next().unwrap();
        if !first.is_ascii_lowercase() {
            return false;
        }
        s.chars().all(|c| c.is_ascii_alphanumeric())
    }

    fn is_pascal_case(s: &str) -> bool {
        if s.is_empty() {
            return false;
        }
        let mut chars = s.chars();
        let first = chars.next().unwrap();
        if !first.is_ascii_uppercase() {
            return false;
        }
        s.chars().all(|c| c.is_ascii_alphanumeric())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum LayoutNode {
    Dir {
        #[serde(default)]
        children: HashMap<String, LayoutNode>,
        #[serde(default)]
        optional: bool,
        #[serde(default)]
        required: bool,
        #[serde(default)]
        strict: bool,
        #[serde(default)]
        max_depth: Option<usize>,
    },
    File {
        pattern: Option<String>,
        #[serde(default)]
        optional: bool,
        #[serde(default)]
        required: bool,
        #[serde(default)]
        case: Option<CaseStyle>,
    },
    Param {
        name: String,
        case: CaseStyle,
        child: Box<LayoutNode>,
    },
    Many {
        case: Option<CaseStyle>,
        child: Box<LayoutNode>,
        #[serde(default)]
        max: Option<usize>,
    },
    Recursive {
        #[serde(default = "default_max_depth")]
        max_depth: usize,
        child: Box<LayoutNode>,
    },
    Either {
        variants: Vec<LayoutNode>,
    },
}

fn default_max_depth() -> usize {
    10
}

impl Default for LayoutNode {
    fn default() -> Self {
        Self::Dir {
            children: HashMap::new(),
            optional: false,
            required: false,
            strict: false,
            max_depth: None,
        }
    }
}

impl LayoutNode {
    pub fn dir(children: HashMap<String, LayoutNode>) -> Self {
        Self::Dir {
            children,
            optional: false,
            required: false,
            strict: false,
            max_depth: None,
        }
    }

    pub fn dir_strict(children: HashMap<String, LayoutNode>) -> Self {
        Self::Dir {
            children,
            optional: false,
            required: false,
            strict: true,
            max_depth: None,
        }
    }

    pub fn file() -> Self {
        Self::File {
            pattern: None,
            optional: false,
            required: false,
            case: None,
        }
    }

    pub fn file_with_pattern(pattern: &str) -> Self {
        Self::File {
            pattern: Some(pattern.to_string()),
            optional: false,
            required: false,
            case: None,
        }
    }

    pub fn file_with_case(pattern: Option<&str>, case: CaseStyle) -> Self {
        Self::File {
            pattern: pattern.map(|s| s.to_string()),
            optional: false,
            required: false,
            case: Some(case),
        }
    }

    pub fn optional(mut self) -> Self {
        match &mut self {
            LayoutNode::Dir { optional, .. } => *optional = true,
            LayoutNode::File { optional, .. } => *optional = true,
            _ => {}
        }
        self
    }

    pub fn required(mut self) -> Self {
        match &mut self {
            LayoutNode::Dir { required, .. } => *required = true,
            LayoutNode::File { required, .. } => *required = true,
            _ => {}
        }
        self
    }

    pub fn strict(mut self) -> Self {
        if let LayoutNode::Dir { strict, .. } = &mut self {
            *strict = true;
        }
        self
    }

    pub fn with_max_depth(mut self, depth: usize) -> Self {
        if let LayoutNode::Dir { max_depth, .. } = &mut self {
            *max_depth = Some(depth);
        }
        self
    }

    pub fn param(name: &str, case: CaseStyle, child: LayoutNode) -> Self {
        Self::Param {
            name: name.to_string(),
            case,
            child: Box::new(child),
        }
    }

    pub fn many(case: Option<CaseStyle>, child: LayoutNode) -> Self {
        Self::Many {
            case,
            child: Box::new(child),
            max: None,
        }
    }

    pub fn many_with_max(case: Option<CaseStyle>, child: LayoutNode, max: usize) -> Self {
        Self::Many {
            case,
            child: Box::new(child),
            max: Some(max),
        }
    }

    pub fn recursive(child: LayoutNode) -> Self {
        Self::Recursive {
            max_depth: 10,
            child: Box::new(child),
        }
    }

    pub fn recursive_with_depth(max_depth: usize, child: LayoutNode) -> Self {
        Self::Recursive {
            max_depth,
            child: Box::new(child),
        }
    }

    pub fn either(variants: Vec<LayoutNode>) -> Self {
        Self::Either { variants }
    }

    pub fn is_optional(&self) -> bool {
        match self {
            LayoutNode::Dir { optional, .. } => *optional,
            LayoutNode::File { optional, .. } => *optional,
            LayoutNode::Param { .. } => false,
            LayoutNode::Many { .. } => false,
            LayoutNode::Recursive { .. } => false,
            LayoutNode::Either { .. } => false,
        }
    }

    pub fn is_required(&self) -> bool {
        match self {
            LayoutNode::Dir { required, .. } => *required,
            LayoutNode::File { required, .. } => *required,
            _ => false,
        }
    }

    pub fn is_strict(&self) -> bool {
        matches!(self, LayoutNode::Dir { strict: true, .. })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RulesConfig {
    #[serde(default)]
    pub forbid_paths: Vec<String>,
    #[serde(default)]
    pub forbid_names: Vec<String>,
    #[serde(default)]
    pub ignore_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BoundariesConfig {
    pub modules: String,
    pub public_api: String,
    #[serde(default)]
    pub forbid_deep_imports: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DepsAllowRule {
    pub from: String,
    pub to: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct DepsConfig {
    #[serde(default)]
    pub allow: Vec<DepsAllowRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DependencyRule {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MirrorConfig {
    pub source: String,
    pub target: String,
    pub pattern: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WhenRequirement {
    pub requires: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ConfigIR {
    #[serde(default)]
    pub mode: Mode,
    pub layout: Option<LayoutNode>,
    #[serde(default)]
    pub rules: RulesConfig,
    pub boundaries: Option<BoundariesConfig>,
    pub deps: Option<DepsConfig>,
    #[serde(default)]
    pub ignore: Vec<String>,
    #[serde(default = "default_use_gitignore")]
    pub use_gitignore: bool,
    #[serde(default)]
    pub workspaces: Vec<String>,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    #[serde(default)]
    pub mirror: Vec<MirrorConfig>,
    #[serde(default)]
    pub when: HashMap<String, WhenRequirement>,
    #[serde(default)]
    pub extends: Option<String>,
}

fn default_use_gitignore() -> bool {
    true
}

impl ConfigIR {
    pub fn new(layout: LayoutNode) -> Self {
        Self {
            mode: Mode::default(),
            layout: Some(layout),
            rules: RulesConfig::default(),
            boundaries: None,
            deps: None,
            ignore: Vec::new(),
            use_gitignore: true,
            workspaces: Vec::new(),
            dependencies: HashMap::new(),
            mirror: Vec::new(),
            when: HashMap::new(),
            extends: None,
        }
    }

    pub fn merge(&mut self, base: ConfigIR) {
        if self.mode == Mode::default() && base.mode != Mode::default() {
            self.mode = base.mode;
        }

        if self.layout.is_none() {
            self.layout = base.layout;
        }

        self.rules.forbid_paths.extend(base.rules.forbid_paths);
        self.rules.forbid_names.extend(base.rules.forbid_names);
        self.rules.ignore_paths.extend(base.rules.ignore_paths);

        if self.boundaries.is_none() {
            self.boundaries = base.boundaries;
        }

        if let Some(base_deps) = base.deps {
            if let Some(ref mut deps) = self.deps {
                deps.allow.extend(base_deps.allow);
            } else {
                self.deps = Some(base_deps);
            }
        }

        self.ignore.extend(base.ignore);

        if self.workspaces.is_empty() {
            self.workspaces = base.workspaces;
        }

        for (k, v) in base.dependencies {
            self.dependencies.entry(k).or_insert(v);
        }

        self.mirror.extend(base.mirror);

        for (k, v) in base.when {
            self.when.entry(k).or_insert(v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kebab_case_validation() {
        assert!(CaseStyle::Kebab.validate("my-module"));
        assert!(CaseStyle::Kebab.validate("billing"));
        assert!(CaseStyle::Kebab.validate("auth-v2"));
        assert!(!CaseStyle::Kebab.validate("MyModule"));
        assert!(!CaseStyle::Kebab.validate("my_module"));
        assert!(!CaseStyle::Kebab.validate("-invalid"));
        assert!(!CaseStyle::Kebab.validate("invalid-"));
        assert!(!CaseStyle::Kebab.validate("in--valid"));
    }

    #[test]
    fn test_snake_case_validation() {
        assert!(CaseStyle::Snake.validate("my_module"));
        assert!(CaseStyle::Snake.validate("billing"));
        assert!(CaseStyle::Snake.validate("auth_v2"));
        assert!(!CaseStyle::Snake.validate("MyModule"));
        assert!(!CaseStyle::Snake.validate("my-module"));
        assert!(!CaseStyle::Snake.validate("_invalid"));
        assert!(!CaseStyle::Snake.validate("invalid_"));
        assert!(!CaseStyle::Snake.validate("in__valid"));
    }

    #[test]
    fn test_camel_case_validation() {
        assert!(CaseStyle::Camel.validate("myModule"));
        assert!(CaseStyle::Camel.validate("billing"));
        assert!(CaseStyle::Camel.validate("authV2"));
        assert!(!CaseStyle::Camel.validate("MyModule"));
        assert!(!CaseStyle::Camel.validate("my-module"));
        assert!(!CaseStyle::Camel.validate("my_module"));
    }

    #[test]
    fn test_pascal_case_validation() {
        assert!(CaseStyle::Pascal.validate("MyModule"));
        assert!(CaseStyle::Pascal.validate("Billing"));
        assert!(CaseStyle::Pascal.validate("AuthV2"));
        assert!(!CaseStyle::Pascal.validate("myModule"));
        assert!(!CaseStyle::Pascal.validate("my-module"));
        assert!(!CaseStyle::Pascal.validate("my_module"));
    }

    #[test]
    fn test_layout_node_builders() {
        let file = LayoutNode::file();
        assert!(matches!(
            file,
            LayoutNode::File {
                pattern: None,
                optional: false,
                ..
            }
        ));

        let optional_file = LayoutNode::file().optional();
        assert!(matches!(
            optional_file,
            LayoutNode::File {
                pattern: None,
                optional: true,
                ..
            }
        ));

        let dir = LayoutNode::dir(HashMap::new());
        assert!(matches!(
            dir,
            LayoutNode::Dir {
                optional: false,
                ..
            }
        ));
    }

    #[test]
    fn test_any_case_validation() {
        assert!(CaseStyle::Any.validate("anything"));
        assert!(CaseStyle::Any.validate("ANYTHING"));
        assert!(CaseStyle::Any.validate("Any-Thing_123"));
    }

    #[test]
    fn test_empty_string_validation() {
        assert!(!CaseStyle::Kebab.validate(""));
        assert!(!CaseStyle::Snake.validate(""));
        assert!(!CaseStyle::Camel.validate(""));
        assert!(!CaseStyle::Pascal.validate(""));
        assert!(CaseStyle::Any.validate(""));
    }

    #[test]
    fn test_file_with_pattern() {
        let file = LayoutNode::file_with_pattern("*.ts");
        match file {
            LayoutNode::File {
                pattern, optional, ..
            } => {
                assert_eq!(pattern, Some("*.ts".to_string()));
                assert!(!optional);
            }
            _ => panic!("Expected File variant"),
        }
    }

    #[test]
    fn test_param_node() {
        let child = LayoutNode::file();
        let param = LayoutNode::param("module", CaseStyle::Kebab, child);
        match param {
            LayoutNode::Param { name, case, .. } => {
                assert_eq!(name, "module");
                assert_eq!(case, CaseStyle::Kebab);
            }
            _ => panic!("Expected Param variant"),
        }
    }

    #[test]
    fn test_many_node() {
        let child = LayoutNode::file();
        let many = LayoutNode::many(Some(CaseStyle::Snake), child);
        match many {
            LayoutNode::Many { case, .. } => {
                assert_eq!(case, Some(CaseStyle::Snake));
            }
            _ => panic!("Expected Many variant"),
        }
    }

    #[test]
    fn test_optional_dir() {
        let dir = LayoutNode::dir(HashMap::new()).optional();
        assert!(dir.is_optional());
    }

    #[test]
    fn test_config_ir_new() {
        let layout = LayoutNode::dir(HashMap::new());
        let config = ConfigIR::new(layout);
        assert_eq!(config.mode, Mode::Strict);
        assert!(config.rules.forbid_paths.is_empty());
        assert!(config.boundaries.is_none());
    }

    #[test]
    fn test_mode_default() {
        assert_eq!(Mode::default(), Mode::Strict);
    }

    #[test]
    fn test_kebab_with_numbers() {
        assert!(CaseStyle::Kebab.validate("v1"));
        assert!(CaseStyle::Kebab.validate("api-v2"));
        assert!(CaseStyle::Kebab.validate("module-123"));
    }

    #[test]
    fn test_snake_with_numbers() {
        assert!(CaseStyle::Snake.validate("v1"));
        assert!(CaseStyle::Snake.validate("api_v2"));
        assert!(CaseStyle::Snake.validate("module_123"));
    }

    #[test]
    fn test_recursive_node() {
        let child = LayoutNode::dir(HashMap::new());
        let recursive = LayoutNode::recursive(child);
        match recursive {
            LayoutNode::Recursive { max_depth, .. } => {
                assert_eq!(max_depth, 10);
            }
            _ => panic!("Expected Recursive variant"),
        }
    }

    #[test]
    fn test_recursive_with_custom_depth() {
        let child = LayoutNode::dir(HashMap::new());
        let recursive = LayoutNode::recursive_with_depth(5, child);
        match recursive {
            LayoutNode::Recursive { max_depth, .. } => {
                assert_eq!(max_depth, 5);
            }
            _ => panic!("Expected Recursive variant"),
        }
    }

    #[test]
    fn test_either_node() {
        let file = LayoutNode::file();
        let dir = LayoutNode::dir(HashMap::new());
        let either = LayoutNode::either(vec![file, dir]);
        match either {
            LayoutNode::Either { variants } => {
                assert_eq!(variants.len(), 2);
            }
            _ => panic!("Expected Either variant"),
        }
    }

    #[test]
    fn test_config_with_ignore() {
        let layout = LayoutNode::dir(HashMap::new());
        let config = ConfigIR::new(layout);
        assert!(config.ignore.is_empty());
        assert!(config.use_gitignore);
    }

    #[test]
    fn test_rules_with_ignore_paths() {
        let rules = RulesConfig {
            forbid_paths: vec!["**/utils/**".to_string()],
            forbid_names: vec![],
            ignore_paths: vec!["**/node_modules/**".to_string()],
        };
        assert_eq!(rules.ignore_paths.len(), 1);
    }
}
