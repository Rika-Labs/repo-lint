use repo_lint::config::ConfigParser;

const COMPLEX_NEXTJS_CONFIG: &str = r#"
import {
	defineConfig,
	dir,
	either,
	file,
	many,
	opt,
	param,
	recursive,
} from '@rikalabs/repo-lint'

const nextjsRouteFiles = dir({
	'page.tsx': opt(file()),
	'layout.tsx': opt(file()),
	'loading.tsx': opt(file()),
	'error.tsx': opt(file()),
	'not-found.tsx': opt(file()),
	'template.tsx': opt(file()),
	'route.ts': opt(file()),
	'globals.css': opt(file()),
	'favicon.ico': opt(file()),
})

const nextjsAppRouter = dir({
	$routes: recursive(
		param({ case: 'any' }, nextjsRouteFiles),
	),
	'page.tsx': opt(file()),
	'layout.tsx': opt(file()),
	'loading.tsx': opt(file()),
	'error.tsx': opt(file()),
	'not-found.tsx': opt(file()),
	'globals.css': opt(file()),
	'favicon.ico': opt(file()),
})

const featureModule = dir({
	components: opt(
		dir({
			$any: many(file('*.tsx')),
		}),
	),
	hooks: opt(dir({ $any: many(file('*.ts')) })),
	server: opt(dir({ $any: many(file('*.ts')) })),
	schemas: opt(dir({ $any: many(file('*.ts')) })),
	'types.ts': opt(file()),
	'constants.ts': opt(file()),
	'utils.ts': opt(file()),
})

const apiModule = dir({
	'service.ts': opt(file()),
	'repo.ts': opt(file()),
	'schemas.ts': opt(file()),
	'errors.ts': opt(file()),
	'types.ts': opt(file()),
	'model.ts': opt(file()),
	'controller.ts': opt(file()),
})

const testModule = dir({
	'service.test.ts': opt(file()),
	'repo.test.ts': opt(file()),
	'controller.test.ts': opt(file()),
	'service.effect.test.ts': opt(file()),
	'controller.effect.test.ts': opt(file()),
	$any: many(file('*.test.ts')),
})

const nextjsApp = dir({
	src: dir({
		app: nextjsAppRouter,
		features: opt(
			dir({
				$domain: param({ case: 'kebab' }, featureModule),
			}),
		),
		lib: opt(dir({ $any: many(file('*.ts')) })),
		server: opt(dir({ $any: many(file('*.ts')) })),
		styles: opt(dir({ $any: many(file('*.css')) })),
		config: opt(dir({ $any: many(file('*.ts')) })),
		types: opt(dir({ $any: many(file('*.ts')) })),
		hooks: opt(dir({ $any: many(file('*.ts')) })),
		components: opt(dir({ $any: many(file('*.tsx')) })),
	}),
	tests: opt(
		dir({
			app: opt(
				dir({
					$routes: recursive(param({ case: 'kebab' }, dir({ $any: many(file('*.test.tsx')) }))),
				}),
			),
			features: opt(
				dir({
					$domain: param(
						{ case: 'kebab' },
						dir({
							components: opt(dir({ $any: many(file('*.test.tsx')) })),
							hooks: opt(dir({ $any: many(file('*.test.ts')) })),
							server: opt(dir({ $any: many(file('*.test.ts')) })),
							schemas: opt(dir({ $any: many(file('*.test.ts')) })),
						}),
					),
				}),
			),
			$any: many(file('*.test.{ts,tsx}')),
		}),
	),
	public: opt(dir({ $any: many(file('*')) })),
	e2e: opt(dir({ $any: many(file('*.{ts,spec.ts}')) })),
	'package.json': file(),
	'tsconfig.json': file(),
	'README.md': opt(file()),
	'next.config.ts': opt(file()),
	'postcss.config.mjs': opt(file()),
	'playwright.config.ts': opt(file()),
})

const elysiaApi = dir({
	src: dir({
		modules: opt(
			dir({
				$domain: param({ case: 'kebab' }, apiModule),
			}),
		),
		domains: opt(
			dir({
				$domain: param({ case: 'kebab' }, apiModule),
			}),
		),
		db: opt(
			dir({
				schema: opt(dir({ $any: many(file('*.ts')) })),
				'types.ts': opt(file()),
			}),
		),
		auth: opt(dir({ $any: many(file('*.ts')) })),
		trpc: opt(
			dir({
				routers: opt(dir({ $any: many(file('*.ts')) })),
				'context.ts': opt(file()),
				'root-router.ts': opt(file()),
			}),
		),
		plugins: opt(dir({ $any: many(file('*.ts')) })),
		middleware: opt(dir({ $any: many(file('*.ts')) })),
		integrations: opt(dir({ $any: many(file('*.ts')) })),
		lib: opt(dir({ $any: many(file('*.ts')) })),
		'index.ts': file(),
		'env.ts': opt(file()),
		'server.ts': opt(file()),
		'app.ts': opt(file()),
		'runtime.ts': opt(file()),
		'elysia.d.ts': opt(file()),
	}),
	tests: opt(
		dir({
			modules: opt(
				dir({
					$domain: param({ case: 'kebab' }, testModule),
				}),
			),
			domains: opt(
				dir({
					$domain: param({ case: 'kebab' }, testModule),
				}),
			),
			'runtime.test.ts': opt(file()),
			$any: many(file('*.test.ts')),
		}),
	),
	e2e: opt(dir({ $any: many(file('*.ts')) })),
	drizzle: opt(dir({ $any: many(file('*')) })),
	'package.json': file(),
	'tsconfig.json': file(),
	'README.md': opt(file()),
	'drizzle.config.ts': opt(file()),
	'playwright.config.ts': opt(file()),
})

const sharedPackage = dir({
	src: dir({
		'index.ts': file(),
		components: opt(
			dir({
				$any: many(file('*.tsx')),
				$subdirs: recursive(param({ case: 'kebab' }, dir({ $any: many(file('*.{ts,tsx}')) }))),
			}),
		),
		hooks: opt(dir({ $any: many(file('*.ts')) })),
		lib: opt(dir({ $any: many(file('*.ts')) })),
		config: opt(dir({ $any: many(file('*.ts')) })),
		types: opt(dir({ $any: many(file('*.ts')) })),
		effect: opt(dir({ $any: many(file('*.ts')) })),
		server: opt(dir({ $any: many(file('*.ts')) })),
		client: opt(dir({ $any: many(file('*.ts')) })),
		providers: opt(dir({ $any: many(file('*.ts')) })),
		schema: opt(dir({ $any: many(file('*.ts')) })),
		columns: opt(dir({ $any: many(file('*.ts')) })),
		queries: opt(dir({ $any: many(file('*.ts')) })),
		errors: opt(dir({ $any: many(file('*.ts')) })),
		middleware: opt(dir({ $any: many(file('*.ts')) })),
		plugins: opt(dir({ $any: many(file('*.ts')) })),
		routers: opt(dir({ $any: many(file('*.ts')) })),
		validation: opt(dir({ $any: many(file('*.ts')) })),
		utils: opt(dir({ $any: many(file('*.ts')) })),
		react: opt(dir({ $any: many(file('*.{ts,tsx}')) })),
		actions: opt(dir({ $any: many(file('*.ts')) })),
		ai: opt(dir({ $any: many(file('*.ts')) })),
		api: opt(dir({ $any: many(file('*.ts')) })),
		auth: opt(dir({ $any: many(file('*.{ts,tsx}')) })),
		fonts: opt(dir({ $any: many(file('*.ts')) })),
		layout: opt(dir({ $any: many(file('*.tsx')) })),
		metadata: opt(dir({ $any: many(file('*.ts')) })),
		routes: opt(dir({ $any: many(file('*.ts')) })),
		agents: opt(dir({ $any: many(file('*.ts')) })),
		memory: opt(dir({ $any: many(file('*.ts')) })),
		mastra: opt(dir({ $any: many(file('*.ts')) })),
		tools: opt(
			dir({
				$any: many(file('*.ts')),
				$subdirs: recursive(param({ case: 'kebab' }, dir({ $any: many(file('*.ts')) }))),
			}),
		),
		workflows: opt(
			dir({
				$any: many(file('*.ts')),
				templates: opt(dir({ $any: many(file('*.ts')) })),
			}),
		),
		templates: opt(dir({ $any: many(file('*.{ts,tsx}')) })),
		$any: many(file('*.ts')),
	}),
	tests: opt(
		dir({
			$any: many(file('*.test.{ts,tsx}')),
		}),
	),
	'package.json': file(),
	'tsconfig.json': file(),
	'README.md': opt(file()),
	'AGENTS.md': opt(file()),
	'components.json': opt(file()),
})

export default defineConfig({
	mode: 'strict',

	ignore: ['.git', 'node_modules', '.next', 'dist', '.turbo', 'test-results', '.vercel', 'out'],

	layout: dir({
		apps: dir({
			sentinel: nextjsApp,
			steward: either(nextjsApp, dir({ app: nextjsAppRouter, $any: many(file('*')) })),
			'sentinel-api': elysiaApi,
			'steward-api': elysiaApi,
			mastra: dir({
				src: dir({ $any: many(file('*.ts')) }),
				'package.json': file(),
				'tsconfig.json': file(),
			}),
		}),
		packages: dir({
			$pkg: param({ case: 'kebab' }, sharedPackage),
		}),
		docs: dir({
			'best-practices': dir({
				$any: many(file('*.md')),
			}),
			$any: many(file('*.md')),
		}),
		scripts: dir({
			$any: many(file('*.{ts,json}')),
			db: opt(dir({ $any: many(file('*.ts')) })),
			lib: opt(dir({ $any: many(file('*.ts')) })),
		}),
		'.github': dir({
			workflows: dir({ $any: many(file('*.yml')) }),
		}),
		'.husky': dir({
			_: dir({ $any: many(file('*')) }),
			$any: many(file('*')),
		}),
		'.claude': opt(dir({ $any: many(file('*')) })),
		'.cursor': opt(dir({ $any: many(file('*')) })),
		'package.json': file(),
		'bun.lock': file(),
		'biome.json': file(),
		'turbo.json': file(),
		'tsconfig.json': file(),
		'README.md': file(),
		'AGENTS.md': file(),
		'CLAUDE.md': opt(file()),
		'docker-compose.yml': opt(file()),
		'lefthook.yml': opt(file()),
		'knip.ts': opt(file()),
		'.dependency-cruiser.cjs': opt(file()),
		'.ls-lint.yml': opt(file()),
		'.oxlintrc.json': opt(file()),
		'.tool-versions': opt(file()),
		'.gitignore': file(),
		'.env.example': opt(file()),
		'.env.local': opt(file()),
		'commitlint.config.ts': opt(file()),
		'playwright.config.ts': opt(file()),
		'repo-lint.config.ts': file(),
	}),

	rules: {
		forbidPaths: ['**/utils/**', '**/*.bak'],
		forbidNames: ['temp', 'new', 'copy', 'tmp'],
		ignorePaths: ['**/.turbo/**', '**/dist/**', '**/.vercel/**'],
	},
})
"#;

#[test]
fn test_parse_complex_nextjs_config() {
    let parser = ConfigParser::new();
    let result = parser.parse_string(COMPLEX_NEXTJS_CONFIG, "repo-lint.config.ts");
    assert!(
        result.is_ok(),
        "Failed to parse complex config: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_config_with_variable_references() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, opt, param, recursive } from '@rikalabs/repo-lint'

const routeFiles = dir({
    'page.tsx': opt(file()),
    'layout.tsx': opt(file()),
})

const appRouter = dir({
    $routes: recursive(param({ case: 'kebab' }, routeFiles)),
})

export default defineConfig({
    layout: dir({
        app: appRouter,
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse config with variable references: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_config_with_nested_variable_references() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, opt, param, many } from '@rikalabs/repo-lint'

const innerFiles = dir({
    'index.ts': file(),
    $any: many(file('*.ts')),
})

const middleDir = dir({
    components: opt(innerFiles),
    hooks: opt(innerFiles),
})

const outerDir = dir({
    $module: param({ case: 'kebab' }, middleDir),
})

export default defineConfig({
    layout: dir({
        src: outerDir,
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse config with nested variable references: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_config_with_either_and_variable() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, opt, either, many } from '@rikalabs/repo-lint'

const simpleApp = dir({
    'index.ts': file(),
})

const complexApp = dir({
    src: dir({
        'index.ts': file(),
    }),
    'package.json': file(),
})

export default defineConfig({
    layout: dir({
        apps: dir({
            myApp: either(simpleApp, complexApp),
        }),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse config with either and variable: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_recursive_with_options_object() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, opt, param, recursive } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        app: dir({
            $routes: recursive(
                { maxDepth: 5 },
                param({ case: 'kebab' }, dir({
                    'page.tsx': opt(file()),
                }))
            ),
        }),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse recursive with options object: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_recursive_without_options() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, opt, param, recursive } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        app: dir({
            $routes: recursive(
                param({ case: 'kebab' }, dir({
                    'page.tsx': opt(file()),
                }))
            ),
        }),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse recursive without options: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_many_with_options() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, many } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        src: dir({
            $component: many({ case: 'pascal' }, file('*.tsx')),
        }),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse many with options: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_many_without_options() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, many } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        src: dir({
            $any: many(file('*.ts')),
        }),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse many without options: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_deeply_nested_structure() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, opt, param, many, recursive } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        level1: dir({
            level2: dir({
                level3: dir({
                    level4: dir({
                        level5: dir({
                            level6: dir({
                                level7: dir({
                                    level8: dir({
                                        level9: dir({
                                            level10: dir({
                                                'deep.ts': file(),
                                            }),
                                        }),
                                    }),
                                }),
                            }),
                        }),
                    }),
                }),
            }),
        }),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse deeply nested structure: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_all_case_styles() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, param } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        kebab: dir({ $k: param({ case: 'kebab' }, file()) }),
        snake: dir({ $s: param({ case: 'snake' }, file()) }),
        camel: dir({ $c: param({ case: 'camel' }, file()) }),
        pascal: dir({ $p: param({ case: 'pascal' }, file()) }),
        any: dir({ $a: param({ case: 'any' }, file()) }),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse all case styles: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_file_patterns() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, many } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        src: dir({
            $ts: many(file('*.ts')),
            $tsx: many(file('*.tsx')),
            $css: many(file('*.css')),
            $json: many(file('*.json')),
            $md: many(file('*.md')),
            $multi: many(file('*.{ts,tsx,js,jsx}')),
            $test: many(file('*.test.{ts,tsx}')),
            $spec: many(file('*.spec.ts')),
        }),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse file patterns: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_ignore_config() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file } from '@rikalabs/repo-lint'

export default defineConfig({
    ignore: ['.git', 'node_modules', '.next', 'dist', '.turbo'],
    layout: dir({
        src: dir({ 'index.ts': file() }),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse ignore config: {:?}",
        result.err()
    );
    let ir = result.unwrap();
    assert_eq!(ir.ignore.len(), 5);
}

#[test]
fn test_parse_use_gitignore_true() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file } from '@rikalabs/repo-lint'

export default defineConfig({
    useGitignore: true,
    layout: dir({
        src: dir({ 'index.ts': file() }),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse useGitignore true: {:?}",
        result.err()
    );
    let ir = result.unwrap();
    assert!(ir.use_gitignore);
}

#[test]
fn test_parse_use_gitignore_false() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file } from '@rikalabs/repo-lint'

export default defineConfig({
    useGitignore: false,
    layout: dir({
        src: dir({ 'index.ts': file() }),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse useGitignore false: {:?}",
        result.err()
    );
    let ir = result.unwrap();
    assert!(!ir.use_gitignore);
}

#[test]
fn test_parse_rules_with_ignore_paths() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        src: dir({ 'index.ts': file() }),
    }),
    rules: {
        forbidPaths: ['**/utils/**'],
        forbidNames: ['temp', 'tmp'],
        ignorePaths: ['**/.turbo/**', '**/dist/**', '**/.vercel/**'],
    },
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse rules with ignorePaths: {:?}",
        result.err()
    );
    let ir = result.unwrap();
    assert_eq!(ir.rules.ignore_paths.len(), 3);
}

#[test]
fn test_parse_mode_strict() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file } from '@rikalabs/repo-lint'

export default defineConfig({
    mode: 'strict',
    layout: dir({
        src: dir({ 'index.ts': file() }),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse mode strict: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_mode_warn() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file } from '@rikalabs/repo-lint'

export default defineConfig({
    mode: 'warn',
    layout: dir({
        src: dir({ 'index.ts': file() }),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse mode warn: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_empty_dir() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({}),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse empty dir: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_dir_without_args() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir(),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse dir without args: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_file_without_pattern() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        'index.ts': file(),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse file without pattern: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_file_with_pattern() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        $any: file('*.ts'),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse file with pattern: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_opt_file() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, opt } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        'README.md': opt(file()),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse opt file: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_opt_dir() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, opt } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        tests: opt(dir({
            'test.ts': file(),
        })),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse opt dir: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_param_with_name() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, param } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        $module: param({ case: 'kebab', name: 'module' }, dir({
            'index.ts': file(),
        })),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse param with name: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_either_multiple_variants() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, either } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        config: either(
            file('config.ts'),
            file('config.js'),
            file('config.json'),
            dir({ 'index.ts': file() }),
        ),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse either multiple variants: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_either_two_variants() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, either } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        app: either(file(), dir({})),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse either two variants: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_special_characters_in_filenames() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, opt } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        '.gitignore': file(),
        '.env.local': opt(file()),
        '.env.example': opt(file()),
        '_app.tsx': opt(file()),
        '__tests__': opt(dir({})),
        '[slug]': opt(dir({})),
        '[[...catchAll]]': opt(dir({})),
        '(group)': opt(dir({})),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse special characters in filenames: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_dotfiles_and_dotdirs() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, opt, many } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        '.github': dir({
            workflows: dir({ $any: many(file('*.yml')) }),
        }),
        '.husky': dir({
            '_': dir({ $any: many(file('*')) }),
        }),
        '.vscode': opt(dir({
            'settings.json': opt(file()),
        })),
        '.claude': opt(dir({ $any: many(file('*')) })),
        '.cursor': opt(dir({ $any: many(file('*')) })),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse dotfiles and dotdirs: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_boundaries_config() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        src: dir({ 'index.ts': file() }),
    }),
    boundaries: {
        modules: 'src/modules/*',
        publicApi: 'src/modules/*/index.ts',
        forbidDeepImports: true,
    },
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse boundaries config: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_deps_config() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        src: dir({ 'index.ts': file() }),
    }),
    deps: {
        allow: [
            { from: 'src/app/**', to: ['src/lib/**', 'src/utils/**'] },
        ],
    },
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse deps config: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_full_monorepo_config() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, opt, param, many, recursive, either } from '@rikalabs/repo-lint'

export default defineConfig({
    mode: 'strict',
    ignore: ['.git', 'node_modules', 'dist', '.turbo'],
    useGitignore: true,
    layout: dir({
        apps: dir({
            $app: param({ case: 'kebab' }, dir({
                src: dir({
                    app: dir({
                        $routes: recursive(param({ case: 'kebab' }, dir({
                            'page.tsx': opt(file()),
                            'layout.tsx': opt(file()),
                        }))),
                    }),
                }),
                'package.json': file(),
            })),
        }),
        packages: dir({
            $pkg: param({ case: 'kebab' }, dir({
                src: dir({
                    'index.ts': file(),
                    $any: many(file('*.ts')),
                }),
                'package.json': file(),
            })),
        }),
        '.github': dir({
            workflows: dir({ $any: many(file('*.yml')) }),
        }),
        'package.json': file(),
        'turbo.json': file(),
    }),
    rules: {
        forbidPaths: ['**/utils/**'],
        forbidNames: ['temp'],
        ignorePaths: ['**/.turbo/**'],
    },
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse full monorepo config: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_config_with_comments() {
    let parser = ConfigParser::new();
    let config = r#"
// This is a comment
import { defineConfig, dir, file } from '@rikalabs/repo-lint'

/* Multi-line
   comment */
export default defineConfig({
    // Mode setting
    mode: 'strict',
    layout: dir({
        // Source directory
        src: dir({
            'index.ts': file(), // Main entry
        }),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse config with comments: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_trailing_commas() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, } from '@rikalabs/repo-lint'

export default defineConfig({
    mode: 'strict',
    layout: dir({
        src: dir({
            'index.ts': file(),
        }),
    }),
    rules: {
        forbidPaths: ['**/utils/**',],
        forbidNames: ['temp', 'tmp',],
    },
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse trailing commas: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_single_quotes() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file } from '@rikalabs/repo-lint'

export default defineConfig({
    mode: 'strict',
    layout: dir({
        src: dir({
            'index.ts': file(),
        }),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse single quotes: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_double_quotes() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file } from "@rikalabs/repo-lint"

export default defineConfig({
    mode: "strict",
    layout: dir({
        src: dir({
            "index.ts": file(),
        }),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse double quotes: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_mixed_quotes() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file } from '@rikalabs/repo-lint'

export default defineConfig({
    mode: "strict",
    layout: dir({
        src: dir({
            'index.ts': file(),
            "utils.ts": file(),
        }),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse mixed quotes: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_max_depth_number() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, opt, param, recursive } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        app: dir({
            $routes: recursive({ maxDepth: 20 }, param({ case: 'kebab' }, dir({
                'page.tsx': opt(file()),
            }))),
        }),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse maxDepth number: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_complex_glob_patterns() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, many } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        src: dir({
            $components: many(file('*.{ts,tsx,js,jsx}')),
            $tests: many(file('*.{test,spec}.{ts,tsx}')),
            $stories: many(file('*.stories.{ts,tsx}')),
        }),
    }),
    rules: {
        forbidPaths: ['**/*.{bak,tmp,temp}', '**/node_modules/**'],
        ignorePaths: ['**/.{git,turbo,next}/**'],
    },
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse complex glob patterns: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_unicode_filenames() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, opt } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        'README.md': file(),
        'LÉEME.md': opt(file()),
        '说明.md': opt(file()),
        'документация': opt(dir({})),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse unicode filenames: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_error_invalid_syntax() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        src: dir({
            'index.ts': file()
        // missing closing braces
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(result.is_err(), "Should fail on invalid syntax");
}

#[test]
fn test_parse_error_missing_layout() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file } from '@rikalabs/repo-lint'

export default defineConfig({
    mode: 'strict',
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(result.is_ok(), "Should allow missing layout (optional now)");
}

#[test]
fn test_parse_error_invalid_mode() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file } from '@rikalabs/repo-lint'

export default defineConfig({
    mode: 'invalid',
    layout: dir({}),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(result.is_err(), "Should fail on invalid mode");
}

#[test]
fn test_parse_error_invalid_case_style() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, param } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        $mod: param({ case: 'invalid' }, dir({})),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(result.is_err(), "Should fail on invalid case style");
}

#[test]
fn test_parse_variable_used_multiple_times() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, opt } from '@rikalabs/repo-lint'

const commonFiles = dir({
    'index.ts': file(),
    'types.ts': opt(file()),
})

export default defineConfig({
    layout: dir({
        moduleA: commonFiles,
        moduleB: commonFiles,
        moduleC: commonFiles,
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse variable used multiple times: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_variable_in_recursive() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, opt, param, recursive } from '@rikalabs/repo-lint'

const routeFiles = dir({
    'page.tsx': opt(file()),
    'layout.tsx': opt(file()),
    'loading.tsx': opt(file()),
})

const routeParam = param({ case: 'kebab' }, routeFiles)

export default defineConfig({
    layout: dir({
        app: dir({
            $routes: recursive(routeParam),
        }),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse variable in recursive: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_variable_in_either() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, either } from '@rikalabs/repo-lint'

const fileVariant = file('*.ts')
const dirVariant = dir({ 'index.ts': file() })

export default defineConfig({
    layout: dir({
        config: either(fileVariant, dirVariant),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse variable in either: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_variable_in_many() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, many } from '@rikalabs/repo-lint'

const tsFile = file('*.ts')

export default defineConfig({
    layout: dir({
        src: dir({
            $any: many(tsFile),
        }),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse variable in many: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_variable_in_opt() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, opt } from '@rikalabs/repo-lint'

const readme = file()

export default defineConfig({
    layout: dir({
        'README.md': opt(readme),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse variable in opt: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_chained_variables() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, opt, param } from '@rikalabs/repo-lint'

const indexFile = file()
const srcDir = dir({ 'index.ts': indexFile })
const moduleDir = param({ case: 'kebab' }, srcDir)
const modulesDir = dir({ $module: moduleDir })

export default defineConfig({
    layout: dir({
        modules: modulesDir,
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse chained variables: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_error_unknown_variable() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        src: unknownVariable,
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(result.is_err(), "Should fail on unknown variable");
}

#[test]
fn test_parse_complex_recursive_nested_in_either() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, opt, param, recursive, either } from '@rikalabs/repo-lint'

const pageFiles = dir({
    'page.tsx': opt(file()),
    'layout.tsx': opt(file()),
})

const routeWithRecursive = dir({
    $routes: recursive(param({ case: 'kebab' }, pageFiles)),
})

const simpleApp = dir({ 'index.ts': file() })

export default defineConfig({
    layout: dir({
        app: either(routeWithRecursive, simpleApp),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse complex recursive nested in either: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_either_nested_in_recursive() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, opt, param, recursive, either } from '@rikalabs/repo-lint'

const fileOrDir = either(file('page.tsx'), dir({ 'index.tsx': file() }))

export default defineConfig({
    layout: dir({
        app: dir({
            $routes: recursive(param({ case: 'kebab' }, dir({
                content: fileOrDir,
            }))),
        }),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse either nested in recursive: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_many_nested_in_recursive() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, many, param, recursive } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        src: dir({
            $routes: recursive(param({ case: 'kebab' }, dir({
                $files: many(file('*.tsx')),
            }))),
        }),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse many nested in recursive: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_multiple_recursive_at_same_level() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, opt, param, recursive } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        app: dir({
            $pages: recursive(param({ case: 'kebab' }, dir({
                'page.tsx': opt(file()),
            }))),
        }),
        api: dir({
            $routes: recursive(param({ case: 'kebab' }, dir({
                'route.ts': opt(file()),
            }))),
        }),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse multiple recursive at same level: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_recursive_with_many_children() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, opt, param, many, recursive } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        components: dir({
            $category: recursive({ maxDepth: 3 }, param({ case: 'kebab' }, dir({
                $component: many({ case: 'pascal' }, dir({
                    'index.tsx': file(),
                    $files: many(file('*.tsx')),
                })),
            }))),
        }),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse recursive with many children: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_extreme_nesting_depth() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, opt, param, many, recursive, either } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        l1: dir({
            l2: dir({
                l3: dir({
                    l4: dir({
                        l5: dir({
                            $routes: recursive({ maxDepth: 15 }, param({ case: 'any' }, dir({
                                content: either(
                                    file('page.tsx'),
                                    dir({
                                        $files: many(file('*.tsx')),
                                        nested: opt(dir({
                                            deep: dir({
                                                'final.ts': file(),
                                            }),
                                        })),
                                    }),
                                ),
                            }))),
                        }),
                    }),
                }),
            }),
        }),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse extreme nesting depth: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_all_config_options_combined() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, opt, param, many, recursive, either } from '@rikalabs/repo-lint'

export default defineConfig({
    mode: 'strict',
    ignore: ['.git', 'node_modules', '.next', 'dist'],
    useGitignore: true,
    layout: dir({
        src: dir({
            app: dir({
                $routes: recursive({ maxDepth: 10 }, param({ case: 'kebab' }, dir({
                    'page.tsx': opt(file()),
                    'layout.tsx': opt(file()),
                }))),
            }),
            components: opt(dir({
                $component: many({ case: 'pascal' }, either(
                    file('*.tsx'),
                    dir({ 'index.tsx': file() }),
                )),
            })),
        }),
        'package.json': file(),
    }),
    rules: {
        forbidPaths: ['**/utils/**', '**/*.bak'],
        forbidNames: ['temp', 'tmp', 'new'],
        ignorePaths: ['**/.turbo/**', '**/dist/**'],
    },
    boundaries: {
        modules: 'src/modules/*',
        publicApi: 'src/modules/*/index.ts',
        forbidDeepImports: true,
    },
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse all config options combined: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_empty_arrays() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file } from '@rikalabs/repo-lint'

export default defineConfig({
    ignore: [],
    layout: dir({
        'index.ts': file(),
    }),
    rules: {
        forbidPaths: [],
        forbidNames: [],
        ignorePaths: [],
    },
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse empty arrays: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_whitespace_variations() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file } from '@rikalabs/repo-lint'

export default defineConfig({
    layout:dir({
        'index.ts':file(),
        'src' : dir( { 'main.ts' : file( ) } ),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse whitespace variations: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_import_alias() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig as dc, dir as d, file as f, opt as o } from '@rikalabs/repo-lint'

export default dc({
    layout: d({
        'README.md': o(f()),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_err(),
        "Should fail on aliased imports (not supported)"
    );
}

#[test]
fn test_parse_large_number_of_files() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, opt } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        'file01.ts': file(),
        'file02.ts': file(),
        'file03.ts': file(),
        'file04.ts': file(),
        'file05.ts': file(),
        'file06.ts': file(),
        'file07.ts': file(),
        'file08.ts': file(),
        'file09.ts': file(),
        'file10.ts': file(),
        'file11.ts': file(),
        'file12.ts': file(),
        'file13.ts': file(),
        'file14.ts': file(),
        'file15.ts': file(),
        'file16.ts': file(),
        'file17.ts': file(),
        'file18.ts': file(),
        'file19.ts': file(),
        'file20.ts': file(),
        'file21.ts': opt(file()),
        'file22.ts': opt(file()),
        'file23.ts': opt(file()),
        'file24.ts': opt(file()),
        'file25.ts': opt(file()),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse large number of files: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_nextjs_catch_all_routes() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, opt, param, recursive } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        app: dir({
            '[slug]': opt(dir({
                'page.tsx': file(),
            })),
            '[[...catchAll]]': opt(dir({
                'page.tsx': file(),
            })),
            '(marketing)': opt(dir({
                about: opt(dir({
                    'page.tsx': file(),
                })),
            })),
            '@modal': opt(dir({
                'default.tsx': file(),
            })),
        }),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse Next.js catch-all routes: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_typescript_specific_patterns() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, opt, many } from '@rikalabs/repo-lint'

export default defineConfig({
    layout: dir({
        src: dir({
            $types: many(file('*.d.ts')),
            $tests: many(file('*.spec.ts')),
            $stories: many(file('*.stories.tsx')),
        }),
        '@types': opt(dir({
            $any: many(file('*.d.ts')),
        })),
        '__tests__': opt(dir({
            $any: many(file('*.test.{ts,tsx}')),
        })),
        '__mocks__': opt(dir({
            $any: many(file('*.ts')),
        })),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse TypeScript-specific patterns: {:?}",
        result.err()
    );
}

#[test]
fn test_parse_monorepo_with_workspaces() {
    let parser = ConfigParser::new();
    let config = r#"
import { defineConfig, dir, file, opt, param, many } from '@rikalabs/repo-lint'

const packageStructure = dir({
    src: dir({
        'index.ts': file(),
        $any: many(file('*.ts')),
    }),
    'package.json': file(),
    'tsconfig.json': file(),
    'README.md': opt(file()),
})

export default defineConfig({
    layout: dir({
        apps: dir({
            $app: param({ case: 'kebab' }, packageStructure),
        }),
        packages: dir({
            $pkg: param({ case: 'kebab' }, packageStructure),
        }),
        libs: opt(dir({
            $lib: param({ case: 'kebab' }, packageStructure),
        })),
        tools: opt(dir({
            $tool: param({ case: 'kebab' }, packageStructure),
        })),
        'package.json': file(),
        'pnpm-workspace.yaml': opt(file()),
        'turbo.json': opt(file()),
        'nx.json': opt(file()),
    }),
})
"#;
    let result = parser.parse_string(config, "test.ts");
    assert!(
        result.is_ok(),
        "Failed to parse monorepo with workspaces: {:?}",
        result.err()
    );
}
