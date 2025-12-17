use std::collections::HashMap;
use std::path::{Path, PathBuf};
use xxhash_rust::xxh3::xxh3_64;

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub path: PathBuf,
    pub content_hash: u64,
    pub mtime: u64,
    pub violations: Vec<String>,
}

pub struct IncrementalCache {
    entries: HashMap<PathBuf, CacheEntry>,
    cache_file: PathBuf,
}

impl IncrementalCache {
    pub fn new(cache_dir: &Path) -> Self {
        let cache_file = cache_dir.join(".repo-lint-cache");
        Self {
            entries: HashMap::new(),
            cache_file,
        }
    }

    pub fn load(&mut self) -> Result<(), std::io::Error> {
        if !self.cache_file.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&self.cache_file)?;
        for line in content.lines() {
            if let Some((path, rest)) = line.split_once('\t') {
                let parts: Vec<&str> = rest.split('\t').collect();
                if parts.len() >= 2 {
                    if let (Ok(hash), Ok(mtime)) = (parts[0].parse::<u64>(), parts[1].parse::<u64>()) {
                        let violations: Vec<String> = if parts.len() > 2 {
                            parts[2..].iter().map(|s| s.to_string()).collect()
                        } else {
                            Vec::new()
                        };

                        self.entries.insert(
                            PathBuf::from(path),
                            CacheEntry {
                                path: PathBuf::from(path),
                                content_hash: hash,
                                mtime,
                                violations,
                            },
                        );
                    }
                }
            }
        }

        Ok(())
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let mut content = String::new();
        for (path, entry) in &self.entries {
            content.push_str(&format!(
                "{}\t{}\t{}\t{}\n",
                path.display(),
                entry.content_hash,
                entry.mtime,
                entry.violations.join("\t")
            ));
        }
        std::fs::write(&self.cache_file, content)
    }

    pub fn is_valid(&self, path: &Path, content: &[u8]) -> bool {
        if let Some(entry) = self.entries.get(path) {
            let current_hash = xxh3_64(content);
            return entry.content_hash == current_hash;
        }
        false
    }

    pub fn get_cached_violations(&self, path: &Path) -> Option<&[String]> {
        self.entries.get(path).map(|e| e.violations.as_slice())
    }

    pub fn update(&mut self, path: PathBuf, content: &[u8], violations: Vec<String>) {
        let hash = xxh3_64(content);
        let mtime = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        self.entries.insert(
            path.clone(),
            CacheEntry {
                path,
                content_hash: hash,
                mtime,
                violations,
            },
        );
    }

    pub fn invalidate(&mut self, path: &Path) {
        self.entries.remove(path);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_is_valid() {
        let temp = TempDir::new().unwrap();
        let mut cache = IncrementalCache::new(temp.path());

        let path = PathBuf::from("test.ts");
        let content = b"const x = 1;";

        cache.update(path.clone(), content, vec![]);

        assert!(cache.is_valid(&path, content));
        assert!(!cache.is_valid(&path, b"const x = 2;"));
    }

    #[test]
    fn test_cache_violations() {
        let temp = TempDir::new().unwrap();
        let mut cache = IncrementalCache::new(temp.path());

        let path = PathBuf::from("test.ts");
        let content = b"const x = 1;";
        let violations = vec!["error1".to_string(), "error2".to_string()];

        cache.update(path.clone(), content, violations.clone());

        let cached = cache.get_cached_violations(&path);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), &violations[..]);
    }

    #[test]
    fn test_cache_save_load() {
        let temp = TempDir::new().unwrap();

        {
            let mut cache = IncrementalCache::new(temp.path());
            cache.update(
                PathBuf::from("test.ts"),
                b"content",
                vec!["violation".to_string()],
            );
            cache.save().unwrap();
        }

        {
            let mut cache = IncrementalCache::new(temp.path());
            cache.load().unwrap();
            assert!(cache.entries.contains_key(&PathBuf::from("test.ts")));
        }
    }
}
