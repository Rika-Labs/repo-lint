use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub is_dir: bool,
}

pub struct Walker {
    root: PathBuf,
    respect_gitignore: bool,
    custom_ignores: Vec<String>,
    max_depth: Option<usize>,
}

impl Walker {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            respect_gitignore: true,
            custom_ignores: Vec::new(),
            max_depth: None,
        }
    }

    pub fn respect_gitignore(mut self, respect: bool) -> Self {
        self.respect_gitignore = respect;
        self
    }

    pub fn add_ignore(mut self, pattern: &str) -> Self {
        self.custom_ignores.push(pattern.to_string());
        self
    }

    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    pub fn walk(&self) -> Vec<FileEntry> {
        let mut builder = WalkBuilder::new(&self.root);

        builder
            .git_ignore(self.respect_gitignore)
            .git_global(self.respect_gitignore)
            .git_exclude(self.respect_gitignore)
            .hidden(false)
            .parents(true)
            .ignore(true);

        if let Some(depth) = self.max_depth {
            builder.max_depth(Some(depth));
        }

        for pattern in &self.custom_ignores {
            builder.add_custom_ignore_filename(pattern);
        }

        let entries: Vec<FileEntry> = builder
            .build()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let path = entry.path().to_path_buf();
                let relative_path = path.strip_prefix(&self.root).ok()?.to_path_buf();

                if relative_path.as_os_str().is_empty() {
                    return None;
                }

                Some(FileEntry {
                    path,
                    relative_path,
                    is_dir: entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false),
                })
            })
            .collect();

        entries
    }

    pub fn walk_parallel<F>(&self, callback: F)
    where
        F: Fn(FileEntry) + Send + Sync,
    {
        let mut builder = WalkBuilder::new(&self.root);

        builder
            .git_ignore(self.respect_gitignore)
            .git_global(self.respect_gitignore)
            .git_exclude(self.respect_gitignore)
            .hidden(false)
            .parents(true)
            .ignore(true)
            .threads(num_cpus::get());

        if let Some(depth) = self.max_depth {
            builder.max_depth(Some(depth));
        }

        let root = self.root.clone();

        builder.build_parallel().run(|| {
            let root = root.clone();
            let callback = &callback;
            Box::new(move |entry| {
                if let Ok(entry) = entry {
                    let abs_path = entry.path().to_path_buf();
                    if let Ok(relative_path) = abs_path.strip_prefix(&root) {
                        let relative_path = relative_path.to_path_buf();
                        if !relative_path.as_os_str().is_empty() {
                            callback(FileEntry {
                                path: abs_path,
                                relative_path,
                                is_dir: entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false),
                            });
                        }
                    }
                }
                ignore::WalkState::Continue
            })
        });
    }

    pub fn walk_and_process<F, T>(&self, processor: F) -> Vec<T>
    where
        F: Fn(&Path) -> Vec<T> + Send + Sync,
        T: Send + 'static,
    {
        let (sender, receiver) = crossbeam_channel::unbounded::<Vec<T>>();

        let mut builder = WalkBuilder::new(&self.root);

        builder
            .git_ignore(self.respect_gitignore)
            .git_global(self.respect_gitignore)
            .git_exclude(self.respect_gitignore)
            .hidden(false)
            .parents(true)
            .ignore(true)
            .threads(num_cpus::get());

        if let Some(depth) = self.max_depth {
            builder.max_depth(Some(depth));
        }

        let root = self.root.clone();

        builder.build_parallel().run(|| {
            let root = root.clone();
            let processor = &processor;
            let sender = sender.clone();

            Box::new(move |entry| {
                if let Ok(entry) = entry {
                    if entry.file_type().map(|ft| !ft.is_dir()).unwrap_or(false) {
                        if let Ok(relative_path) = entry.path().strip_prefix(&root) {
                            if !relative_path.as_os_str().is_empty() {
                                let items = processor(relative_path);
                                if !items.is_empty() {
                                    let _ = sender.send(items);
                                }
                            }
                        }
                    }
                }
                ignore::WalkState::Continue
            })
        });

        drop(sender);
        
        let mut results = Vec::with_capacity(4096);
        for batch in receiver {
            results.extend(batch);
        }
        results
    }

    pub fn walk_changed(&self, base_ref: &str) -> Result<Vec<FileEntry>, std::io::Error> {
        let output = std::process::Command::new("git")
            .args(["diff", "--name-only", base_ref])
            .current_dir(&self.root)
            .output()?;

        if !output.status.success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let files: Vec<FileEntry> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| {
                let relative_path = PathBuf::from(line);
                let path = self.root.join(&relative_path);
                let is_dir = path.is_dir();
                FileEntry {
                    path,
                    relative_path,
                    is_dir,
                }
            })
            .collect();

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    fn create_test_structure() -> TempDir {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        fs::create_dir_all(root.join("src/services/billing")).unwrap();
        fs::write(root.join("src/services/billing/index.ts"), "").unwrap();
        fs::create_dir_all(root.join("src/services/auth")).unwrap();
        fs::write(root.join("src/services/auth/index.ts"), "").unwrap();
        fs::write(root.join("README.md"), "").unwrap();

        temp
    }

    #[test]
    fn test_walk_basic() {
        let temp = create_test_structure();
        let walker = Walker::new(temp.path());

        let entries = walker.walk();
        assert!(!entries.is_empty());

        let paths: Vec<String> = entries
            .iter()
            .map(|e| e.relative_path.to_string_lossy().to_string())
            .collect();

        assert!(paths.iter().any(|p| p.contains("billing")));
        assert!(paths.iter().any(|p| p.contains("auth")));
    }

    #[test]
    fn test_walk_max_depth() {
        let temp = create_test_structure();
        let walker = Walker::new(temp.path()).max_depth(2);

        let entries = walker.walk();
        let max_depth = entries
            .iter()
            .map(|e| e.relative_path.components().count())
            .max()
            .unwrap_or(0);

        assert!(max_depth <= 2);
    }
}
