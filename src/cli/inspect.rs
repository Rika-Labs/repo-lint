use clap::Args;
use std::path::PathBuf;

use crate::config::ConfigParser;
use crate::engine::{FileMatcher, MatchResult};

#[derive(Args)]
pub struct InspectArgs {
    #[command(subcommand)]
    pub inspect_type: InspectType,
}

#[derive(clap::Subcommand)]
pub enum InspectType {
    Layout,
    Path { path: String },
    Rule { rule_id: String },
    Deps { path: String },
}

pub struct InspectCommand;

impl InspectCommand {
    pub fn run(
        args: &InspectArgs,
        config_path: &str,
        json_output: bool,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let config_file = PathBuf::from(config_path);
        if !config_file.exists() {
            return Err(format!("Config file not found: {}", config_path).into());
        }

        let parser = ConfigParser::new();
        let config = parser.parse_file(&config_file)?;

        match &args.inspect_type {
            InspectType::Layout => {
                if json_output {
                    let layout_json = serde_json::to_string_pretty(&config.layout)?;
                    println!("{}", layout_json);
                } else {
                    Self::print_layout_tree(&config.layout, "", true);
                }
            }
            InspectType::Path { path } => {
                let matcher = FileMatcher::new(&config)?;
                let explanation = matcher.explain_path(&PathBuf::from(path));

                if json_output {
                    let json_output = serde_json::json!({
                        "path": path,
                        "allowed": matches!(
                            explanation.match_result,
                            MatchResult::Allowed | MatchResult::AllowedParam { .. } | MatchResult::AllowedMany { .. }
                        ),
                        "match_result": format!("{:?}", explanation.match_result),
                        "expected_children": explanation.expected_children.iter().map(|c| {
                            serde_json::json!({
                                "name": c.name,
                                "is_dir": c.is_dir,
                                "optional": c.optional,
                                "is_param": c.is_param,
                            })
                        }).collect::<Vec<_>>(),
                    });
                    println!("{}", serde_json::to_string_pretty(&json_output)?);
                } else {
                    println!("Path: {}", path);
                    println!();
                    match &explanation.match_result {
                        MatchResult::Allowed => {
                            println!("  Status: ALLOWED");
                        }
                        MatchResult::AllowedParam { name, value } => {
                            println!("  Status: ALLOWED (param {}={})", name, value);
                        }
                        MatchResult::AllowedMany { values } => {
                            println!("  Status: ALLOWED (many: {:?})", values);
                        }
                        MatchResult::Denied { reason, attempts } => {
                            println!("  Status: DENIED");
                            println!("  Reason: {}", reason);
                            if !attempts.is_empty() {
                                println!("  Tried to match:");
                                for attempt in attempts {
                                    let status = if attempt.matched { "✓" } else { "✗" };
                                    let reason = attempt
                                        .reason
                                        .as_ref()
                                        .map(|r| format!(" ({})", r))
                                        .unwrap_or_default();
                                    println!("    {} {}{}", status, attempt.pattern, reason);
                                }
                            }
                        }
                        MatchResult::NotInLayout {
                            nearest_valid,
                            attempts,
                        } => {
                            println!("  Status: NOT IN LAYOUT");
                            if let Some(nearest) = nearest_valid {
                                println!("  Nearest valid parent: {}", nearest);
                            }
                            if !attempts.is_empty() {
                                println!("  Tried to match:");
                                for attempt in attempts {
                                    let status = if attempt.matched { "✓" } else { "✗" };
                                    let reason = attempt
                                        .reason
                                        .as_ref()
                                        .map(|r| format!(" ({})", r))
                                        .unwrap_or_default();
                                    println!("    {} {}{}", status, attempt.pattern, reason);
                                }
                            }
                        }
                        MatchResult::MissingRequired { expected } => {
                            println!("  Status: MISSING REQUIRED CHILDREN");
                            println!("  Expected: {:?}", expected);
                        }
                    }

                    if !explanation.expected_children.is_empty() {
                        println!();
                        println!("  Expected children at this path:");
                        for child in &explanation.expected_children {
                            let kind = if child.is_dir { "dir" } else { "file" };
                            let opt = if child.optional { " (optional)" } else { "" };
                            let param = if child.is_param { " [param]" } else { "" };
                            println!("    - {} [{}]{}{}", child.name, kind, opt, param);
                        }
                    }
                }
            }
            InspectType::Rule { rule_id } => {
                let rule_info = Self::get_rule_info(rule_id);
                if json_output {
                    println!("{}", serde_json::to_string_pretty(&rule_info)?);
                } else {
                    println!("Rule: {}", rule_id);
                    println!();
                    println!("  Description: {}", rule_info.description);
                    println!(
                        "  Auto-fix: {}",
                        if rule_info.auto_fix { "yes" } else { "no" }
                    );
                    if !rule_info.examples.is_empty() {
                        println!();
                        println!("  Examples:");
                        for example in &rule_info.examples {
                            println!("    - {}", example);
                        }
                    }
                }
            }
            InspectType::Deps { path } => {
                println!(
                    "Dependency inspection for '{}' is not yet implemented.",
                    path
                );
                println!("This feature will be available in M4.");
            }
        }

        Ok(0)
    }

    fn print_layout_tree(node: &crate::config::LayoutNode, prefix: &str, is_last: bool) {
        let _connector = if is_last { "└── " } else { "├── " };
        let _extension = if is_last { "    " } else { "│   " };

        match node {
            crate::config::LayoutNode::Dir {
                children,
                optional,
                required,
                strict,
                ..
            } => {
                if !prefix.is_empty() {
                    let mut flags = Vec::new();
                    if *optional {
                        flags.push("opt");
                    }
                    if *required {
                        flags.push("req");
                    }
                    if *strict {
                        flags.push("strict");
                    }
                    let flag_str = if flags.is_empty() {
                        String::new()
                    } else {
                        format!(" ({})", flags.join(", "))
                    };
                    println!("{}[dir]{}", prefix, flag_str);
                }

                let mut sorted_children: Vec<_> = children.iter().collect();
                sorted_children.sort_by(|a, b| a.0.cmp(b.0));

                for (i, (name, child)) in sorted_children.iter().enumerate() {
                    let is_last_child = i == sorted_children.len() - 1;
                    let child_connector = if is_last_child {
                        "└── "
                    } else {
                        "├── "
                    };
                    let child_extension = if is_last_child { "    " } else { "│   " };

                    print!("{}{}{}", prefix, child_connector, name);
                    Self::print_layout_tree(
                        child,
                        &format!("{}{}", prefix, child_extension),
                        is_last_child,
                    );
                }
            }
            crate::config::LayoutNode::File {
                pattern,
                optional,
                required,
                case,
            } => {
                let mut flags = Vec::new();
                if *optional {
                    flags.push("opt".to_string());
                }
                if *required {
                    flags.push("req".to_string());
                }
                if let Some(c) = case {
                    flags.push(format!("case: {:?}", c));
                }
                let flag_str = if flags.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", flags.join(", "))
                };
                let pat = pattern
                    .as_ref()
                    .map(|p| format!(" [{}]", p))
                    .unwrap_or_default();
                println!("{}{}", pat, flag_str);
            }
            crate::config::LayoutNode::Param { name, case, child } => {
                println!(" [param: {}, case: {:?}]", name, case);
                Self::print_layout_tree(child, &format!("{}    ", prefix), true);
            }
            crate::config::LayoutNode::Many { case, child, max } => {
                let case_str = case
                    .as_ref()
                    .map(|c| format!(", case: {:?}", c))
                    .unwrap_or_default();
                let max_str = max.map(|m| format!(", max: {}", m)).unwrap_or_default();
                println!(" [many{}{}]", case_str, max_str);
                Self::print_layout_tree(child, &format!("{}    ", prefix), true);
            }
            crate::config::LayoutNode::Recursive { max_depth, child } => {
                println!(" [recursive, max_depth: {}]", max_depth);
                Self::print_layout_tree(child, &format!("{}    ", prefix), true);
            }
            crate::config::LayoutNode::Either { variants } => {
                println!(" [either, {} variants]", variants.len());
                for (i, variant) in variants.iter().enumerate() {
                    Self::print_layout_tree(
                        variant,
                        &format!("{}    ", prefix),
                        i == variants.len() - 1,
                    );
                }
            }
        }
    }

    fn get_rule_info(rule_id: &str) -> RuleInfo {
        match rule_id {
            "layout" => RuleInfo {
                id: "layout".to_string(),
                description: "Enforces filesystem structure matches the defined layout".to_string(),
                auto_fix: true,
                examples: vec![
                    "Files must exist in paths defined by dir/file DSL".to_string(),
                    "Param constraints (e.g., kebab-case) must be satisfied".to_string(),
                ],
            },
            "forbidPaths" => RuleInfo {
                id: "forbidPaths".to_string(),
                description: "Forbids files/directories matching specified glob patterns"
                    .to_string(),
                auto_fix: false,
                examples: vec![
                    "**/utils/** - forbid utils directories".to_string(),
                    "**/*.bak - forbid backup files".to_string(),
                ],
            },
            "forbidNames" => RuleInfo {
                id: "forbidNames".to_string(),
                description: "Forbids files/directories with specific names".to_string(),
                auto_fix: true,
                examples: vec![
                    "temp - forbid 'temp' named files/dirs".to_string(),
                    "new - forbid 'new' named files/dirs".to_string(),
                ],
            },
            _ => RuleInfo {
                id: rule_id.to_string(),
                description: "Unknown rule".to_string(),
                auto_fix: false,
                examples: vec![],
            },
        }
    }
}

#[derive(serde::Serialize)]
struct RuleInfo {
    id: String,
    description: String,
    auto_fix: bool,
    examples: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_rule_info_known_rule() {
        let info = InspectCommand::get_rule_info("layout");
        assert_eq!(info.id, "layout");
        assert!(info.auto_fix);
    }

    #[test]
    fn test_get_rule_info_unknown_rule() {
        let info = InspectCommand::get_rule_info("unknown");
        assert_eq!(info.description, "Unknown rule");
        assert!(!info.auto_fix);
    }
}
