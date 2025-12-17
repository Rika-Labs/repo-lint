use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Strict,
    Warn,
}

impl Default for Mode {
    fn default() -> Self {
        Self::Strict
    }
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
            && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            && !s.starts_with('-')
            && !s.ends_with('-')
            && !s.contains("--")
    }

    fn is_snake_case(s: &str) -> bool {
        !s.is_empty()
            && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
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
        children: HashMap<String, LayoutNode>,
        #[serde(default)]
        optional: bool,
    },
    File {
        pattern: Option<String>,
        #[serde(default)]
        optional: bool,
    },
    Param {
        name: String,
        case: CaseStyle,
        child: Box<LayoutNode>,
    },
    Many {
        case: Option<CaseStyle>,
        child: Box<LayoutNode>,
    },
}

impl LayoutNode {
    pub fn dir(children: HashMap<String, LayoutNode>) -> Self {
        Self::Dir {
            children,
            optional: false,
        }
    }

    pub fn file() -> Self {
        Self::File {
            pattern: None,
            optional: false,
        }
    }

    pub fn file_with_pattern(pattern: &str) -> Self {
        Self::File {
            pattern: Some(pattern.to_string()),
            optional: false,
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
        }
    }

    pub fn is_optional(&self) -> bool {
        match self {
            LayoutNode::Dir { optional, .. } => *optional,
            LayoutNode::File { optional, .. } => *optional,
            LayoutNode::Param { .. } => false,
            LayoutNode::Many { .. } => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RulesConfig {
    #[serde(default)]
    pub forbid_paths: Vec<String>,
    #[serde(default)]
    pub forbid_names: Vec<String>,
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
pub struct ConfigIR {
    #[serde(default)]
    pub mode: Mode,
    pub layout: LayoutNode,
    #[serde(default)]
    pub rules: RulesConfig,
    pub boundaries: Option<BoundariesConfig>,
    pub deps: Option<DepsConfig>,
}

impl ConfigIR {
    pub fn new(layout: LayoutNode) -> Self {
        Self {
            mode: Mode::default(),
            layout,
            rules: RulesConfig::default(),
            boundaries: None,
            deps: None,
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
        assert!(matches!(file, LayoutNode::File { pattern: None, optional: false }));

        let optional_file = LayoutNode::file().optional();
        assert!(matches!(optional_file, LayoutNode::File { pattern: None, optional: true }));

        let dir = LayoutNode::dir(HashMap::new());
        assert!(matches!(dir, LayoutNode::Dir { optional: false, .. }));
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
            LayoutNode::File { pattern, optional } => {
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
}
