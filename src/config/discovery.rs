use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

const CONFIG_FILENAME: &str = "repo-lint.config.ts";

#[derive(Debug, Clone)]
pub struct DiscoveredConfig {
    pub config_path: PathBuf,
    pub workspace_root: PathBuf,
    pub relative_path: String,
}

pub struct ConfigDiscovery {
    root: PathBuf,
    respect_gitignore: bool,
}

impl ConfigDiscovery {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            respect_gitignore: true,
        }
    }

    pub fn respect_gitignore(mut self, respect: bool) -> Self {
        self.respect_gitignore = respect;
        self
    }

    pub fn discover(&self) -> Vec<DiscoveredConfig> {
        let mut configs = Vec::new();

        let mut builder = WalkBuilder::new(&self.root);
        builder
            .git_ignore(self.respect_gitignore)
            .git_global(self.respect_gitignore)
            .git_exclude(self.respect_gitignore)
            .hidden(false)
            .parents(true)
            .ignore(true);

        for entry in builder.build().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path
                .file_name()
                .map(|n| n == CONFIG_FILENAME)
                .unwrap_or(false)
            {
                let workspace_root = path.parent().unwrap_or(path).to_path_buf();
                let relative_path = workspace_root
                    .strip_prefix(&self.root)
                    .map(|p| p.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_default();

                configs.push(DiscoveredConfig {
                    config_path: path.to_path_buf(),
                    workspace_root,
                    relative_path,
                });
            }
        }

        configs.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        configs
    }

    pub fn discover_with_workspaces(&self, patterns: &[String]) -> Vec<DiscoveredConfig> {
        if patterns.is_empty() {
            return self.discover();
        }

        let mut configs = Vec::new();

        if let Some(root_config) = self.find_root_config() {
            configs.push(root_config);
        }

        for pattern in patterns {
            let glob_pattern = format!("{}/{}", pattern, CONFIG_FILENAME);
            if let Ok(entries) = glob::glob(&self.root.join(&glob_pattern).to_string_lossy()) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let workspace_root = entry.parent().unwrap_or(&entry).to_path_buf();
                    let relative_path = workspace_root
                        .strip_prefix(&self.root)
                        .map(|p| p.to_string_lossy().replace('\\', "/"))
                        .unwrap_or_default();

                    if !configs.iter().any(|c| c.config_path == entry) {
                        configs.push(DiscoveredConfig {
                            config_path: entry,
                            workspace_root,
                            relative_path,
                        });
                    }
                }
            }
        }

        configs.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        configs
    }

    pub fn find_root_config(&self) -> Option<DiscoveredConfig> {
        let config_path = self.root.join(CONFIG_FILENAME);
        if config_path.exists() {
            Some(DiscoveredConfig {
                config_path,
                workspace_root: self.root.clone(),
                relative_path: String::new(),
            })
        } else {
            None
        }
    }

    pub fn filter_by_workspace(
        &self,
        configs: Vec<DiscoveredConfig>,
        workspace_filter: &str,
    ) -> Vec<DiscoveredConfig> {
        if workspace_filter.contains('*') {
            let pattern = format!("{}/{}", workspace_filter, CONFIG_FILENAME);
            let full_pattern = self.root.join(&pattern);
            if let Ok(matcher) = glob::Pattern::new(&full_pattern.to_string_lossy()) {
                return configs
                    .into_iter()
                    .filter(|c| matcher.matches_path(&c.config_path) || c.relative_path.is_empty())
                    .collect();
            }
        }

        configs
            .into_iter()
            .filter(|c| {
                c.relative_path == workspace_filter
                    || c.relative_path.is_empty()
                    || c.relative_path
                        .starts_with(&format!("{}/", workspace_filter))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_config(dir: &Path, content: &str) {
        fs::write(dir.join(CONFIG_FILENAME), content).unwrap();
    }

    fn minimal_config() -> &'static str {
        r#"
import { defineConfig, dir, file } from "@rikalabs/repo-lint";
export default defineConfig({ layout: dir({ "index.ts": file() }) });
"#
    }

    #[test]
    fn test_discover_single_root_config() {
        let temp = TempDir::new().unwrap();
        create_config(temp.path(), minimal_config());

        let discovery = ConfigDiscovery::new(temp.path());
        let configs = discovery.discover();

        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].relative_path, "");
    }

    #[test]
    fn test_discover_workspace_configs() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        create_config(root, minimal_config());
        fs::create_dir_all(root.join("apps/web")).unwrap();
        create_config(&root.join("apps/web"), minimal_config());
        fs::create_dir_all(root.join("apps/api")).unwrap();
        create_config(&root.join("apps/api"), minimal_config());
        fs::create_dir_all(root.join("packages/ui")).unwrap();
        create_config(&root.join("packages/ui"), minimal_config());

        let discovery = ConfigDiscovery::new(root);
        let configs = discovery.discover();

        assert_eq!(configs.len(), 4);
        let paths: Vec<&str> = configs.iter().map(|c| c.relative_path.as_str()).collect();
        assert!(paths.contains(&""));
        assert!(paths.contains(&"apps/api"));
        assert!(paths.contains(&"apps/web"));
        assert!(paths.contains(&"packages/ui"));
    }

    #[test]
    fn test_discover_ignores_gitignored_dirs() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join(".gitignore"), "node_modules/\n").unwrap();
        create_config(root, minimal_config());
        fs::create_dir_all(root.join("node_modules/some-pkg")).unwrap();
        create_config(&root.join("node_modules/some-pkg"), minimal_config());

        let discovery = ConfigDiscovery::new(root);
        let configs = discovery.discover();

        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].relative_path, "");
    }

    #[test]
    fn test_discover_with_explicit_workspaces() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        create_config(root, minimal_config());
        fs::create_dir_all(root.join("apps/web")).unwrap();
        create_config(&root.join("apps/web"), minimal_config());
        fs::create_dir_all(root.join("apps/api")).unwrap();
        create_config(&root.join("apps/api"), minimal_config());
        fs::create_dir_all(root.join("packages/ui")).unwrap();
        create_config(&root.join("packages/ui"), minimal_config());
        fs::create_dir_all(root.join("tools/scripts")).unwrap();
        create_config(&root.join("tools/scripts"), minimal_config());

        let discovery = ConfigDiscovery::new(root);
        let configs =
            discovery.discover_with_workspaces(&["apps/*".to_string(), "packages/*".to_string()]);

        assert_eq!(configs.len(), 4);
        let paths: Vec<&str> = configs.iter().map(|c| c.relative_path.as_str()).collect();
        assert!(paths.contains(&""));
        assert!(paths.contains(&"apps/api"));
        assert!(paths.contains(&"apps/web"));
        assert!(paths.contains(&"packages/ui"));
        assert!(!paths.contains(&"tools/scripts"));
    }

    #[test]
    fn test_find_root_config() {
        let temp = TempDir::new().unwrap();
        create_config(temp.path(), minimal_config());

        let discovery = ConfigDiscovery::new(temp.path());
        let root_config = discovery.find_root_config();

        assert!(root_config.is_some());
        assert_eq!(root_config.unwrap().relative_path, "");
    }

    #[test]
    fn test_find_root_config_missing() {
        let temp = TempDir::new().unwrap();

        let discovery = ConfigDiscovery::new(temp.path());
        let root_config = discovery.find_root_config();

        assert!(root_config.is_none());
    }

    #[test]
    fn test_filter_by_workspace_exact() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        create_config(root, minimal_config());
        fs::create_dir_all(root.join("apps/web")).unwrap();
        create_config(&root.join("apps/web"), minimal_config());
        fs::create_dir_all(root.join("apps/api")).unwrap();
        create_config(&root.join("apps/api"), minimal_config());

        let discovery = ConfigDiscovery::new(root);
        let all_configs = discovery.discover();
        let filtered = discovery.filter_by_workspace(all_configs, "apps/web");

        assert_eq!(filtered.len(), 2);
        let paths: Vec<&str> = filtered.iter().map(|c| c.relative_path.as_str()).collect();
        assert!(paths.contains(&""));
        assert!(paths.contains(&"apps/web"));
    }

    #[test]
    fn test_filter_by_workspace_glob() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        create_config(root, minimal_config());
        fs::create_dir_all(root.join("apps/web")).unwrap();
        create_config(&root.join("apps/web"), minimal_config());
        fs::create_dir_all(root.join("apps/api")).unwrap();
        create_config(&root.join("apps/api"), minimal_config());
        fs::create_dir_all(root.join("packages/ui")).unwrap();
        create_config(&root.join("packages/ui"), minimal_config());

        let discovery = ConfigDiscovery::new(root);
        let all_configs = discovery.discover();
        let filtered = discovery.filter_by_workspace(all_configs, "apps/*");

        assert_eq!(filtered.len(), 3);
        let paths: Vec<&str> = filtered.iter().map(|c| c.relative_path.as_str()).collect();
        assert!(paths.contains(&""));
        assert!(paths.contains(&"apps/web"));
        assert!(paths.contains(&"apps/api"));
        assert!(!paths.contains(&"packages/ui"));
    }

    #[test]
    fn test_discover_empty_repo() {
        let temp = TempDir::new().unwrap();

        let discovery = ConfigDiscovery::new(temp.path());
        let configs = discovery.discover();

        assert!(configs.is_empty());
    }

    #[test]
    fn test_discover_nested_workspace_configs() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        create_config(root, minimal_config());
        fs::create_dir_all(root.join("apps/web/packages/shared")).unwrap();
        create_config(&root.join("apps/web"), minimal_config());
        create_config(&root.join("apps/web/packages/shared"), minimal_config());

        let discovery = ConfigDiscovery::new(root);
        let configs = discovery.discover();

        assert_eq!(configs.len(), 3);
    }
}
