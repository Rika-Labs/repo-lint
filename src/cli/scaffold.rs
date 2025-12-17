use clap::Args;
use std::fs;
use std::path::PathBuf;

use crate::config::{ConfigParser, LayoutNode};

#[derive(Args)]
pub struct ScaffoldArgs {
    #[command(subcommand)]
    pub scaffold_type: ScaffoldType,

    #[arg(long)]
    pub dry_run: bool,
}

#[derive(clap::Subcommand)]
pub enum ScaffoldType {
    Module {
        name: String,
        #[arg(long, default_value = "src/services")]
        base_path: String,
    },
}

pub struct ScaffoldCommand;

#[derive(serde::Serialize)]
pub struct ScaffoldPlan {
    pub actions: Vec<ScaffoldAction>,
}

#[derive(serde::Serialize)]
pub struct ScaffoldAction {
    pub action: String,
    pub path: String,
}

impl ScaffoldCommand {
    pub fn run(
        args: &ScaffoldArgs,
        config_path: &str,
        json_output: bool,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let config_file = PathBuf::from(config_path);
        if !config_file.exists() {
            return Err(format!("Config file not found: {}", config_path).into());
        }

        let parser = ConfigParser::new();
        let config = parser.parse_file(&config_file)?;

        match &args.scaffold_type {
            ScaffoldType::Module { name, base_path } => {
                let plan = Self::plan_module_scaffold(&config.layout, name, base_path)?;

                if json_output {
                    println!("{}", serde_json::to_string_pretty(&plan)?);
                } else if args.dry_run {
                    println!("Scaffold plan for module '{}':", name);
                    for action in &plan.actions {
                        println!("  {} {}", action.action, action.path);
                    }
                } else {
                    for action in &plan.actions {
                        match action.action.as_str() {
                            "mkdir" => {
                                fs::create_dir_all(&action.path)?;
                                println!("Created directory: {}", action.path);
                            }
                            "touch" => {
                                if let Some(parent) = PathBuf::from(&action.path).parent() {
                                    fs::create_dir_all(parent)?;
                                }
                                fs::write(&action.path, "")?;
                                println!("Created file: {}", action.path);
                            }
                            _ => {}
                        }
                    }
                }

                Ok(0)
            }
        }
    }

    fn plan_module_scaffold(
        layout: &LayoutNode,
        module_name: &str,
        base_path: &str,
    ) -> Result<ScaffoldPlan, Box<dyn std::error::Error>> {
        let mut actions = Vec::new();
        let module_path = format!("{}/{}", base_path, module_name);

        actions.push(ScaffoldAction {
            action: "mkdir".to_string(),
            path: module_path.clone(),
        });

        Self::collect_scaffold_actions(layout, &module_path, &mut actions, 0);

        Ok(ScaffoldPlan { actions })
    }

    fn collect_scaffold_actions(
        node: &LayoutNode,
        current_path: &str,
        actions: &mut Vec<ScaffoldAction>,
        depth: usize,
    ) {
        if depth > 10 {
            return;
        }

        match node {
            LayoutNode::Dir { children, .. } => {
                for (name, child) in children {
                    if name.starts_with('$') {
                        continue;
                    }

                    let child_path = format!("{}/{}", current_path, name);

                    match child {
                        LayoutNode::Dir { .. } => {
                            actions.push(ScaffoldAction {
                                action: "mkdir".to_string(),
                                path: child_path.clone(),
                            });
                            Self::collect_scaffold_actions(child, &child_path, actions, depth + 1);
                        }
                        LayoutNode::File { optional, .. } => {
                            if !optional {
                                actions.push(ScaffoldAction {
                                    action: "touch".to_string(),
                                    path: child_path,
                                });
                            }
                        }
                        LayoutNode::Param { child: inner, .. } => {
                            Self::collect_scaffold_actions(inner, &child_path, actions, depth + 1);
                        }
                        LayoutNode::Many { child: inner, .. } => {
                            Self::collect_scaffold_actions(inner, &child_path, actions, depth + 1);
                        }
                    }
                }
            }
            LayoutNode::Param { child, .. } => {
                Self::collect_scaffold_actions(child, current_path, actions, depth + 1);
            }
            LayoutNode::Many { child, .. } => {
                Self::collect_scaffold_actions(child, current_path, actions, depth + 1);
            }
            LayoutNode::File { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_plan_module_scaffold() {
        let mut api_children = HashMap::new();
        api_children.insert("index.ts".to_string(), LayoutNode::file());

        let mut module_children = HashMap::new();
        module_children.insert("api".to_string(), LayoutNode::dir(api_children));

        let layout = LayoutNode::dir(module_children);

        let plan = ScaffoldCommand::plan_module_scaffold(&layout, "billing", "src/services").unwrap();

        assert!(!plan.actions.is_empty());
        assert!(plan.actions.iter().any(|a| a.path.contains("billing")));
    }
}
