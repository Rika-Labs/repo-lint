use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn create_config(dir: &Path, content: &str) {
    fs::write(dir.join("repo-lint.config.ts"), content).unwrap();
}

#[test]
fn test_directory_alias() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let config = r#"
import { defineConfig, directory, file } from "@rikalabs/repo-lint";

export default defineConfig({
    layout: directory({
        src: directory({
            "index.ts": file(),
        }),
    }),
});
"#;
    create_config(root, config);

    let parser = repo_lint::config::ConfigParser::new();
    let result = parser.parse_file(&root.join("repo-lint.config.ts"));
    assert!(result.is_ok());
}

#[test]
fn test_optional_alias() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let config = r#"
import { defineConfig, directory, file, optional } from "@rikalabs/repo-lint";

export default defineConfig({
    layout: directory({
        src: directory({}),
        tests: optional(directory({})),
        "README.md": optional(file()),
    }),
});
"#;
    create_config(root, config);

    let parser = repo_lint::config::ConfigParser::new();
    let result = parser.parse_file(&root.join("repo-lint.config.ts"));
    assert!(result.is_ok());
    let config = result.unwrap();

    if let repo_lint::config::LayoutNode::Dir { children, .. } = &config.layout {
        if let Some(repo_lint::config::LayoutNode::Dir { optional, .. }) = children.get("tests") {
            assert!(*optional);
        } else {
            panic!("Expected tests to be a Dir");
        }
    }
}

#[test]
fn test_required_modifier() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let config = r#"
import { defineConfig, directory, file, required } from "@rikalabs/repo-lint";

export default defineConfig({
    layout: directory({
        "package.json": required(file()),
        src: required(directory({
            "index.ts": file(),
        })),
    }),
});
"#;
    create_config(root, config);

    let parser = repo_lint::config::ConfigParser::new();
    let result = parser.parse_file(&root.join("repo-lint.config.ts"));
    assert!(result.is_ok());
    let config = result.unwrap();

    if let repo_lint::config::LayoutNode::Dir { children, .. } = &config.layout {
        if let Some(repo_lint::config::LayoutNode::File { required, .. }) =
            children.get("package.json")
        {
            assert!(*required);
        }
        if let Some(repo_lint::config::LayoutNode::Dir { required, .. }) = children.get("src") {
            assert!(*required);
        }
    }
}

#[test]
fn test_strict_directory() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let config = r#"
import { defineConfig, directory, file, many } from "@rikalabs/repo-lint";

export default defineConfig({
    layout: directory({
        hooks: directory({
            $hook: many(file("use-*.ts")),
        }, { strict: true }),
    }),
});
"#;
    create_config(root, config);

    let parser = repo_lint::config::ConfigParser::new();
    let result = parser.parse_file(&root.join("repo-lint.config.ts"));
    assert!(result.is_ok());
    let config = result.unwrap();

    if let repo_lint::config::LayoutNode::Dir { children, .. } = &config.layout {
        if let Some(repo_lint::config::LayoutNode::Dir { strict, .. }) = children.get("hooks") {
            assert!(*strict);
        }
    }
}

#[test]
fn test_dir_max_depth() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let config = r#"
import { defineConfig, directory, file, many } from "@rikalabs/repo-lint";

export default defineConfig({
    layout: directory({
        components: directory({
            $component: many(file("*.tsx")),
        }, { maxDepth: 2 }),
    }),
});
"#;
    create_config(root, config);

    let parser = repo_lint::config::ConfigParser::new();
    let result = parser.parse_file(&root.join("repo-lint.config.ts"));
    assert!(result.is_ok());
    let config = result.unwrap();

    if let repo_lint::config::LayoutNode::Dir { children, .. } = &config.layout {
        if let Some(repo_lint::config::LayoutNode::Dir { max_depth, .. }) =
            children.get("components")
        {
            assert_eq!(*max_depth, Some(2));
        }
    }
}

#[test]
fn test_file_with_case() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let config = r#"
import { defineConfig, directory, file, many } from "@rikalabs/repo-lint";

export default defineConfig({
    layout: directory({
        hooks: directory({
            $hook: many(file({ pattern: "use-*.ts", case: "kebab" })),
        }),
    }),
});
"#;
    create_config(root, config);

    let parser = repo_lint::config::ConfigParser::new();
    let result = parser.parse_file(&root.join("repo-lint.config.ts"));
    assert!(result.is_ok());
    let config = result.unwrap();

    if let repo_lint::config::LayoutNode::Dir { children, .. } = &config.layout {
        if let Some(repo_lint::config::LayoutNode::Dir {
            children: hooks_children,
            ..
        }) = children.get("hooks")
        {
            if let Some(repo_lint::config::LayoutNode::Many { child, .. }) =
                hooks_children.get("$hook")
            {
                if let repo_lint::config::LayoutNode::File { case, pattern, .. } = child.as_ref() {
                    assert_eq!(*case, Some(repo_lint::config::CaseStyle::Kebab));
                    assert_eq!(*pattern, Some("use-*.ts".to_string()));
                }
            }
        }
    }
}

#[test]
fn test_many_with_max() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let config = r#"
import { defineConfig, directory, file, many } from "@rikalabs/repo-lint";

export default defineConfig({
    layout: directory({
        components: directory({
            $component: many({ max: 20 }, file("*.tsx")),
        }),
    }),
});
"#;
    create_config(root, config);

    let parser = repo_lint::config::ConfigParser::new();
    let result = parser.parse_file(&root.join("repo-lint.config.ts"));
    assert!(result.is_ok());
    let config = result.unwrap();

    if let repo_lint::config::LayoutNode::Dir { children, .. } = &config.layout {
        if let Some(repo_lint::config::LayoutNode::Dir {
            children: comp_children,
            ..
        }) = children.get("components")
        {
            if let Some(repo_lint::config::LayoutNode::Many { max, .. }) =
                comp_children.get("$component")
            {
                assert_eq!(*max, Some(20));
            }
        }
    }
}

#[test]
fn test_dependencies_config() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let config = r#"
import { defineConfig, directory, file } from "@rikalabs/repo-lint";

export default defineConfig({
    layout: directory({}),
    dependencies: {
        "src/modules/*/service.ts": "tests/modules/*/service.test.ts",
    },
});
"#;
    create_config(root, config);

    let parser = repo_lint::config::ConfigParser::new();
    let result = parser.parse_file(&root.join("repo-lint.config.ts"));
    assert!(result.is_ok());
    let config = result.unwrap();

    assert_eq!(config.dependencies.len(), 1);
    assert_eq!(
        config.dependencies.get("src/modules/*/service.ts"),
        Some(&"tests/modules/*/service.test.ts".to_string())
    );
}

#[test]
fn test_mirror_config() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let config = r#"
import { defineConfig, directory, file } from "@rikalabs/repo-lint";

export default defineConfig({
    layout: directory({}),
    mirror: [
        {
            source: "src/modules/*",
            target: "tests/modules/*",
            pattern: "*.ts -> *.test.ts",
        },
    ],
});
"#;
    create_config(root, config);

    let parser = repo_lint::config::ConfigParser::new();
    let result = parser.parse_file(&root.join("repo-lint.config.ts"));
    assert!(result.is_ok());
    let config = result.unwrap();

    assert_eq!(config.mirror.len(), 1);
    assert_eq!(config.mirror[0].source, "src/modules/*");
    assert_eq!(config.mirror[0].target, "tests/modules/*");
    assert_eq!(config.mirror[0].pattern, "*.ts -> *.test.ts");
}

#[test]
fn test_when_config() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let config = r#"
import { defineConfig, directory, file } from "@rikalabs/repo-lint";

export default defineConfig({
    layout: directory({}),
    when: {
        "controller.ts": { requires: ["model.ts", "service.ts"] },
    },
});
"#;
    create_config(root, config);

    let parser = repo_lint::config::ConfigParser::new();
    let result = parser.parse_file(&root.join("repo-lint.config.ts"));
    assert!(result.is_ok());
    let config = result.unwrap();

    assert_eq!(config.when.len(), 1);
    let req = config.when.get("controller.ts").unwrap();
    assert_eq!(req.requires, vec!["model.ts", "service.ts"]);
}

#[test]
fn test_strict_mode_rejects_unknown_files() {
    use std::path::Path;

    let mut children = std::collections::HashMap::new();
    children.insert(
        "allowed.ts".to_string(),
        repo_lint::config::LayoutNode::file(),
    );

    let layout = repo_lint::config::LayoutNode::Dir {
        children,
        optional: false,
        required: false,
        strict: true,
        max_depth: None,
    };

    let matcher = repo_lint::engine::LayoutMatcher::new(layout);

    let result = matcher.match_path(Path::new("allowed.ts"));
    assert!(matches!(result, repo_lint::engine::MatchResult::Allowed));

    let result = matcher.match_path(Path::new("unknown.ts"));
    assert!(matches!(
        result,
        repo_lint::engine::MatchResult::Denied { .. }
    ));
}

#[test]
fn test_file_case_validation() {
    use std::path::Path;

    let mut children = std::collections::HashMap::new();
    children.insert(
        "$hook".to_string(),
        repo_lint::config::LayoutNode::Many {
            case: None,
            child: Box::new(repo_lint::config::LayoutNode::File {
                pattern: Some("*.ts".to_string()),
                optional: false,
                required: false,
                case: Some(repo_lint::config::CaseStyle::Kebab),
            }),
            max: None,
        },
    );

    let layout = repo_lint::config::LayoutNode::dir(children);
    let matcher = repo_lint::engine::LayoutMatcher::new(layout);

    let result = matcher.match_path(Path::new("use-auth.ts"));
    println!("Result for use-auth.ts: {:?}", result);
    assert!(matches!(
        result,
        repo_lint::engine::MatchResult::AllowedMany { .. }
    ));

    let result = matcher.match_path(Path::new("useAuth.ts"));
    println!("Result for useAuth.ts: {:?}", result);
    assert!(
        matches!(result, repo_lint::engine::MatchResult::Denied { .. }),
        "Expected Denied but got {:?}",
        result
    );
}

#[test]
fn test_combined_features() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let config = r#"
import { defineConfig, directory, file, optional, required, many } from "@rikalabs/repo-lint";

export default defineConfig({
    layout: directory({
        src: required(directory({
            hooks: optional(directory({
                $hook: many(file({ pattern: "use-*.ts", case: "kebab" })),
            }, { strict: true, maxDepth: 2 })),
        })),
        "package.json": required(file()),
        "README.md": optional(file()),
    }),
    dependencies: {
        "src/**/*.ts": "tests/**/*.test.ts",
    },
    when: {
        "service.ts": { requires: ["service.test.ts"] },
    },
});
"#;
    create_config(root, config);

    let parser = repo_lint::config::ConfigParser::new();
    let result = parser.parse_file(&root.join("repo-lint.config.ts"));
    assert!(result.is_ok());
}

#[test]
fn test_deeply_nested_recursive_patterns() {
    use repo_lint::config::{CaseStyle, LayoutNode};
    use repo_lint::engine::{LayoutMatcher, MatchResult};
    use std::collections::HashMap;

    let mut file_children = HashMap::new();
    file_children.insert("page.tsx".to_string(), LayoutNode::file());
    file_children.insert("layout.tsx".to_string(), LayoutNode::file().optional());
    file_children.insert(
        "$any".to_string(),
        LayoutNode::many(None, LayoutNode::file_with_pattern("*")),
    );

    let param_child = LayoutNode::param("segment", CaseStyle::Any, LayoutNode::dir(file_children));

    let recursive_node = LayoutNode::recursive_with_depth(10, param_child);

    let mut app_children = HashMap::new();
    app_children.insert("$routes".to_string(), recursive_node);

    let mut src_children = HashMap::new();
    src_children.insert("app".to_string(), LayoutNode::dir(app_children));

    let mut root_children = HashMap::new();
    root_children.insert("src".to_string(), LayoutNode::dir(src_children));

    let matcher = LayoutMatcher::new(LayoutNode::dir(root_children));

    let is_allowed = |path: &str| -> bool {
        matches!(
            matcher.match_path(Path::new(path)),
            MatchResult::Allowed
                | MatchResult::AllowedParam { .. }
                | MatchResult::AllowedMany { .. }
        )
    };

    assert!(
        is_allowed("src/app/dashboard/page.tsx"),
        "depth 1 should be allowed"
    );

    assert!(
        is_allowed("src/app/dashboard/settings/page.tsx"),
        "depth 2 should be allowed"
    );

    assert!(
        is_allowed("src/app/dashboard/settings/profile/page.tsx"),
        "depth 3 should be allowed"
    );

    assert!(
        is_allowed("src/app/a/b/c/d/e/page.tsx"),
        "depth 5 should be allowed"
    );

    assert!(
        is_allowed("src/app/a/b/c/layout.tsx"),
        "optional layout.tsx at depth 3 should be allowed"
    );

    assert!(
        is_allowed("src/app/dashboard/some-file.ts"),
        "any file via many(*) should be allowed"
    );
}

#[test]
fn test_recursive_with_nested_param_directory() {
    use repo_lint::config::{CaseStyle, LayoutNode};
    use repo_lint::engine::{LayoutMatcher, MatchResult};
    use std::collections::HashMap;

    let mut inner_dir = HashMap::new();
    inner_dir.insert(
        "$any".to_string(),
        LayoutNode::many(None, LayoutNode::file_with_pattern("*")),
    );

    let nested_param =
        LayoutNode::param("nested", CaseStyle::Any, LayoutNode::dir(inner_dir.clone()));

    let mut inner_children = HashMap::new();
    inner_children.insert(
        "$nested".to_string(),
        LayoutNode::recursive_with_depth(10, nested_param),
    );
    inner_children.insert(
        "$any".to_string(),
        LayoutNode::many(None, LayoutNode::file_with_pattern("*")),
    );

    let param_child = LayoutNode::param("app", CaseStyle::Kebab, LayoutNode::dir(inner_children));

    let mut apps_children = HashMap::new();
    apps_children.insert("$app".to_string(), param_child);

    let mut root_children = HashMap::new();
    root_children.insert("apps".to_string(), LayoutNode::dir(apps_children));

    let matcher = LayoutMatcher::new(LayoutNode::dir(root_children));

    let is_allowed = |path: &str| -> bool {
        let result = matcher.match_path(Path::new(path));
        matches!(
            result,
            MatchResult::Allowed
                | MatchResult::AllowedParam { .. }
                | MatchResult::AllowedMany { .. }
        )
    };

    assert!(
        is_allowed("apps/my-app/file.json"),
        "direct file in app dir"
    );
    assert!(
        is_allowed("apps/my-app/drizzle/file.json"),
        "file at depth 1"
    );
    assert!(
        is_allowed("apps/my-app/drizzle/meta/file.json"),
        "file at depth 2"
    );
    assert!(
        is_allowed("apps/my-app/drizzle/meta/deep/file.json"),
        "file at depth 3"
    );
}
