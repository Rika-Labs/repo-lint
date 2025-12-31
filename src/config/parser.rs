use std::collections::HashMap;
use std::path::{Path, PathBuf};
use swc_common::sync::Lrc;
use swc_common::{FileName, SourceMap, Spanned};
use swc_ecma_ast::*;
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};
use thiserror::Error;

use super::ir::{
    BoundariesConfig, CaseStyle, ConfigIR, DepsAllowRule, DepsConfig, LayoutNode, MirrorConfig,
    Mode, RulesConfig, WhenRequirement,
};

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Failed to read config file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Syntax error at {line}:{col}: {message}")]
    SyntaxError {
        line: usize,
        col: usize,
        message: String,
    },
    #[error("Unsupported expression at {line}:{col}: {message}")]
    UnsupportedExpression {
        line: usize,
        col: usize,
        message: String,
    },
    #[error("Missing required field: {0}")]
    MissingField(String),
    #[error("Invalid value for {field}: {message}")]
    InvalidValue { field: String, message: String },
}

use std::cell::RefCell;

pub struct ConfigParser {
    source_map: Lrc<SourceMap>,
    module_exports_cache: RefCell<HashMap<PathBuf, ModuleExports>>,
    workspace_package_cache: RefCell<HashMap<String, Option<PathBuf>>>,
}

#[derive(Clone, Default)]
struct ModuleExports {
    layouts: HashMap<String, LayoutNode>,
    when: HashMap<String, HashMap<String, WhenRequirement>>,
    string_arrays: HashMap<String, Vec<String>>,
    rules: HashMap<String, RulesConfig>,
    mirrors: HashMap<String, Vec<MirrorConfig>>,
}

impl ConfigParser {
    pub fn new() -> Self {
        Self {
            source_map: Lrc::new(SourceMap::default()),
            module_exports_cache: RefCell::new(HashMap::new()),
            workspace_package_cache: RefCell::new(HashMap::new()),
        }
    }

    pub fn parse_file(&self, path: &Path) -> Result<ConfigIR, ParseError> {
        let content = std::fs::read_to_string(path)?;
        let config = self.parse_string(&content, path.to_string_lossy().as_ref())?;

        if let Some(extends_path) = config.extends.clone() {
            let base_path = self.resolve_import(path, &extends_path).ok_or_else(|| {
                ParseError::InvalidValue {
                    field: "extends".to_string(),
                    message: format!("Could not resolve extended config: {}", extends_path),
                }
            })?;
            let base_config = self.parse_file(&base_path)?;
            let mut merged = config;
            merged.merge(base_config);
            return Ok(merged);
        }

        Ok(config)
    }

    pub fn resolve_import(&self, current_file: &Path, specifier: &str) -> Option<PathBuf> {
        let dir = current_file.parent()?;

        // Support @/ as root-relative path
        if let Some(stripped) = specifier.strip_prefix("@/") {
            let mut root = dir;
            let mut current = Some(dir);
            while let Some(d) = current {
                if d.join("repo-lint.config.ts").exists() {
                    root = d;
                }
                current = d.parent();
            }
            if let Some(path) = self.resolve_path(&root.join(stripped)) {
                return Some(path);
            }
        }

        // Try relative path
        if specifier.starts_with('.') {
            if let Some(path) = self.resolve_path(&dir.join(specifier)) {
                return Some(path);
            }
        }

        // Try workspace package resolution for scoped packages (@org/pkg/path)
        if specifier.starts_with('@') && !specifier.starts_with("@/") {
            if let Some(path) = self.resolve_workspace_package(dir, specifier) {
                return Some(path);
            }
        }

        // Try node_modules (search upwards)
        let mut current_dir = Some(dir);
        while let Some(d) = current_dir {
            let node_modules = d.join("node_modules");
            if node_modules.exists() {
                if let Some(path) = self.resolve_path(&node_modules.join(specifier)) {
                    return Some(path);
                }
            }
            current_dir = d.parent();
        }

        None
    }

    fn resolve_workspace_package(&self, start_dir: &Path, specifier: &str) -> Option<PathBuf> {
        // Parse @org/pkg/subpath into parts
        let parts: Vec<&str> = specifier.splitn(3, '/').collect();
        if parts.len() < 2 {
            return None;
        }
        let package_name = format!("{}/{}", parts[0], parts[1]);
        let subpath = if parts.len() > 2 { parts[2] } else { "" };

        // Find monorepo root (has package.json with workspaces)
        let mut root = None;
        let mut current = Some(start_dir);
        while let Some(d) = current {
            let pkg_json = d.join("package.json");
            if pkg_json.exists() {
                if let Ok(content) = std::fs::read_to_string(&pkg_json) {
                    if content.contains("\"workspaces\"") {
                        root = Some(d.to_path_buf());
                        break;
                    }
                }
            }
            current = d.parent();
        }
        let root = root?;

        let cache_key = format!("{}::{}", root.display(), specifier);
        if let Some(cached) = self
            .workspace_package_cache
            .borrow()
            .get(&cache_key)
            .cloned()
        {
            return cached;
        }

        // Search common workspace directories for package.json with matching name
        let search_dirs = ["packages", "apps", "libs"];
        for search_dir in &search_dirs {
            let base = root.join(search_dir);
            if !base.exists() {
                continue;
            }
            if let Some(pkg_path) = self.find_package_in_dir(&base, &package_name, subpath) {
                self.workspace_package_cache
                    .borrow_mut()
                    .insert(cache_key, Some(pkg_path.clone()));
                return Some(pkg_path);
            }
        }

        self.workspace_package_cache
            .borrow_mut()
            .insert(cache_key, None);
        None
    }

    fn find_package_in_dir(
        &self,
        dir: &Path,
        package_name: &str,
        subpath: &str,
    ) -> Option<PathBuf> {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return None;
        };

        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }
            if !file_type.is_dir() {
                continue;
            }
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if matches!(
                    name,
                    "node_modules"
                        | ".git"
                        | ".hg"
                        | ".svn"
                        | ".next"
                        | ".turbo"
                        | ".submodules"
                        | "dist"
                        | "build"
                        | "out"
                        | "target"
                        | "coverage"
                        | "test-results"
                        | ".cache"
                        | ".vercel"
                ) {
                    continue;
                }
            }

            // Check this directory
            let pkg_json = path.join("package.json");
            if pkg_json.exists() {
                if let Ok(content) = std::fs::read_to_string(&pkg_json) {
                    if let Some(name) = self.extract_package_name(&content) {
                        if name == package_name {
                            let target = if subpath.is_empty() {
                                path.join("src")
                            } else {
                                path.join(subpath)
                            };
                            return self.resolve_path(&target);
                        }
                    }
                }
            }

            // Recurse into subdirectories (for nested workspaces like packages/core/*)
            if let Some(found) = self.find_package_in_dir(&path, package_name, subpath) {
                return Some(found);
            }
        }

        None
    }

    fn extract_package_name(&self, content: &str) -> Option<String> {
        // Simple regex-like search for "name": "value" pattern
        // This handles both formatted and minified JSON
        if let Some(start) = content.find("\"name\"") {
            let rest = &content[start..];
            // Find the colon after "name"
            if let Some(colon_pos) = rest.find(':') {
                let after_colon = rest[colon_pos + 1..].trim_start();
                // Now extract the string value
                if after_colon.starts_with('"') {
                    let value_start = 1; // skip opening quote
                    if let Some(end_quote) = after_colon[value_start..].find('"') {
                        return Some(after_colon[value_start..value_start + end_quote].to_string());
                    }
                }
            }
        }
        None
    }

    fn resolve_path(&self, path: &Path) -> Option<PathBuf> {
        // 1. Direct path
        if path.exists() && path.is_file() {
            return Some(path.to_path_buf());
        }
        // 2. Add .ts extension
        let mut with_ext = path.to_path_buf();
        if with_ext.extension().is_none() {
            with_ext.set_extension("ts");
            if with_ext.exists() && with_ext.is_file() {
                return Some(with_ext);
            }
        }
        // 3. Directory index/config
        if path.is_dir() {
            let index = path.join("index.ts");
            if index.exists() && index.is_file() {
                return Some(index);
            }
            let config = path.join("repo-lint.config.ts");
            if config.exists() && config.is_file() {
                return Some(config);
            }
        }
        None
    }

    pub fn parse_string(&self, content: &str, filename: &str) -> Result<ConfigIR, ParseError> {
        let source_file = self.source_map.new_source_file(
            Lrc::new(FileName::Custom(filename.to_string())),
            content.to_string(),
        );

        let lexer = Lexer::new(
            Syntax::Typescript(TsSyntax {
                tsx: false,
                decorators: false,
                dts: false,
                no_early_errors: true,
                disallow_ambiguous_jsx_like: false,
            }),
            EsVersion::Es2022,
            StringInput::from(&*source_file),
            None,
        );

        let mut parser = Parser::new_from(lexer);
        let module = parser.parse_module().map_err(|e| {
            let span = e.span();
            let (line, col) = if span.lo.0 == 0 {
                (0, 0)
            } else {
                let loc = self.source_map.lookup_char_pos(span.lo);
                (loc.line, loc.col_display)
            };
            ParseError::SyntaxError {
                line,
                col,
                message: e.kind().msg().to_string(),
            }
        })?;

        self.extract_config(&module, Path::new(filename))
    }

    fn extract_config(&self, module: &Module, current_path: &Path) -> Result<ConfigIR, ParseError> {
        let mut variables: HashMap<String, &Expr> = HashMap::new();
        let mut imported_layouts: HashMap<String, LayoutNode> = HashMap::new();
        let mut imported_when: HashMap<String, HashMap<String, WhenRequirement>> = HashMap::new();
        let mut imported_string_arrays: HashMap<String, Vec<String>> = HashMap::new();
        let mut imported_rules: HashMap<String, RulesConfig> = HashMap::new();
        let mut imported_mirrors: HashMap<String, Vec<MirrorConfig>> = HashMap::new();

        for item in &module.body {
            match item {
                ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) => {
                    for decl in &var_decl.decls {
                        if let Pat::Ident(ident) = &decl.name {
                            if let Some(init) = &decl.init {
                                variables.insert(ident.sym.to_string(), init.as_ref());
                            }
                        }
                    }
                }
                ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export)) => {
                    if let Decl::Var(var_decl) = &export.decl {
                        for decl in &var_decl.decls {
                            if let Pat::Ident(ident) = &decl.name {
                                if let Some(init) = &decl.init {
                                    variables.insert(ident.sym.to_string(), init.as_ref());
                                }
                            }
                        }
                    }
                }
                ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => {
                    let specifier = import.src.value.as_ref();
                    if let Some(import_path) = self.resolve_import(current_path, specifier) {
                        if let Ok(exports) = self.parse_module_exports(&import_path) {
                            for spec in &import.specifiers {
                                if let ImportSpecifier::Named(named) = spec {
                                    let local = named.local.sym.to_string();
                                    let imported = named
                                        .imported
                                        .as_ref()
                                        .map(|i| match i {
                                            ModuleExportName::Ident(id) => id.sym.to_string(),
                                            ModuleExportName::Str(s) => s.value.to_string(),
                                        })
                                        .unwrap_or_else(|| local.clone());

                                    if let Some(layout) = exports.layouts.get(&imported) {
                                        imported_layouts.insert(local.clone(), layout.clone());
                                    }
                                    if let Some(when) = exports.when.get(&imported) {
                                        imported_when.insert(local.clone(), when.clone());
                                    }
                                    if let Some(arr) = exports.string_arrays.get(&imported) {
                                        imported_string_arrays.insert(local.clone(), arr.clone());
                                    }
                                    if let Some(rules) = exports.rules.get(&imported) {
                                        imported_rules.insert(local.clone(), rules.clone());
                                    }
                                    if let Some(mirror) = exports.mirrors.get(&imported) {
                                        imported_mirrors.insert(local, mirror.clone());
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        for item in &module.body {
            if let ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(export)) = item {
                return self.eval_define_config(
                    &export.expr,
                    &variables,
                    &imported_layouts,
                    &imported_when,
                    &imported_string_arrays,
                    &imported_rules,
                    &imported_mirrors,
                );
            }
        }
        Err(ParseError::MissingField(
            "export default defineConfig(...)".to_string(),
        ))
    }

    fn parse_module_exports(&self, path: &Path) -> Result<ModuleExports, ParseError> {
        if let Some(cached) = self.module_exports_cache.borrow().get(path) {
            return Ok(cached.clone());
        }

        let content = std::fs::read_to_string(path)?;
        let source_file = self.source_map.new_source_file(
            Lrc::new(FileName::Custom(path.to_string_lossy().to_string())),
            content,
        );

        let lexer = Lexer::new(
            Syntax::Typescript(TsSyntax {
                tsx: false,
                decorators: false,
                dts: false,
                no_early_errors: true,
                disallow_ambiguous_jsx_like: false,
            }),
            EsVersion::Es2022,
            StringInput::from(&*source_file),
            None,
        );

        let mut parser = Parser::new_from(lexer);
        let module = parser.parse_module().map_err(|e| ParseError::SyntaxError {
            line: 0,
            col: 0,
            message: e.kind().msg().to_string(),
        })?;

        let mut exports = ModuleExports::default();
        let mut variables: HashMap<String, &Expr> = HashMap::new();
        let mut imported_layouts: HashMap<String, LayoutNode> = HashMap::new();
        let mut imported_when: HashMap<String, HashMap<String, WhenRequirement>> = HashMap::new();

        // First pass: gather local variables and imported exports so exported values can reference them.
        for item in &module.body {
            match item {
                ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) => {
                    for decl in &var_decl.decls {
                        if let Pat::Ident(ident) = &decl.name {
                            if let Some(init) = &decl.init {
                                variables.insert(ident.sym.to_string(), init.as_ref());
                            }
                        }
                    }
                }
                ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export)) => {
                    if let Decl::Var(var_decl) = &export.decl {
                        for decl in &var_decl.decls {
                            if let Pat::Ident(ident) = &decl.name {
                                if let Some(init) = &decl.init {
                                    variables.insert(ident.sym.to_string(), init.as_ref());
                                }
                            }
                        }
                    }
                }
                ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => {
                    let specifier = import.src.value.as_ref();
                    if let Some(import_path) = self.resolve_import(path, specifier) {
                        if let Ok(dep_exports) = self.parse_module_exports(&import_path) {
                            for spec in &import.specifiers {
                                if let ImportSpecifier::Named(named) = spec {
                                    let local = named.local.sym.to_string();
                                    let imported = named
                                        .imported
                                        .as_ref()
                                        .map(|i| match i {
                                            ModuleExportName::Ident(id) => id.sym.to_string(),
                                            ModuleExportName::Str(s) => s.value.to_string(),
                                        })
                                        .unwrap_or_else(|| local.clone());

                                    if let Some(layout) = dep_exports.layouts.get(&imported) {
                                        imported_layouts.insert(local.clone(), layout.clone());
                                    }
                                    if let Some(when) = dep_exports.when.get(&imported) {
                                        imported_when.insert(local, when.clone());
                                    }
                                }
                            }
                        }
                    }
                }
                // Handle re-exports: export { x } from './module'
                ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(named_export)) => {
                    if let Some(src) = &named_export.src {
                        let specifier = src.value.as_ref();
                        if let Some(import_path) = self.resolve_import(path, specifier) {
                            if let Ok(dep_exports) = self.parse_module_exports(&import_path) {
                                for spec in &named_export.specifiers {
                                    if let ExportSpecifier::Named(named) = spec {
                                        let orig_name = match &named.orig {
                                            ModuleExportName::Ident(id) => id.sym.to_string(),
                                            ModuleExportName::Str(s) => s.value.to_string(),
                                        };
                                        let exported_name = named
                                            .exported
                                            .as_ref()
                                            .map(|e| match e {
                                                ModuleExportName::Ident(id) => id.sym.to_string(),
                                                ModuleExportName::Str(s) => s.value.to_string(),
                                            })
                                            .unwrap_or_else(|| orig_name.clone());

                                        if let Some(layout) = dep_exports.layouts.get(&orig_name) {
                                            imported_layouts
                                                .insert(exported_name.clone(), layout.clone());
                                        }
                                        if let Some(when) = dep_exports.when.get(&orig_name) {
                                            imported_when.insert(exported_name, when.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Second pass: evaluate exports.
        for item in &module.body {
            match item {
                ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export)) => {
                    if let Decl::Var(var_decl) = &export.decl {
                        for decl in &var_decl.decls {
                            if let Pat::Ident(ident) = &decl.name {
                                if let Some(init) = &decl.init {
                                    let name = ident.sym.to_string();
                                    if let Ok(layout) =
                                        self.eval_layout_node(init, &variables, &imported_layouts)
                                    {
                                        exports.layouts.insert(name.clone(), layout);
                                        continue;
                                    }

                                    if let Ok(when) =
                                        self.eval_when(init, &variables, &imported_when)
                                    {
                                        exports.when.insert(name.clone(), when);
                                        continue;
                                    }

                                    // Try to evaluate as string array (for ignore, forbidPaths, etc.)
                                    if let Ok(arr) = self.eval_string_array(init) {
                                        exports.string_arrays.insert(name.clone(), arr);
                                        continue;
                                    }

                                    // Try to evaluate as rules config
                                    if let Ok(rules) = self.eval_rules(init, &variables) {
                                        exports.rules.insert(name.clone(), rules);
                                        continue;
                                    }

                                    // Try to evaluate as mirror config
                                    if let Ok(mirror) = self.eval_mirror(init, &variables) {
                                        exports.mirrors.insert(name, mirror);
                                    }
                                }
                            }
                        }
                    }
                }
                // Handle re-exports: export { x } from './module'
                // The layouts/when were already gathered in first pass into imported_layouts/imported_when
                // Now we need to add them to the exports
                ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(named_export)) => {
                    if named_export.src.is_some() {
                        // Re-export from another module
                        for spec in &named_export.specifiers {
                            if let ExportSpecifier::Named(named) = spec {
                                let orig_name = match &named.orig {
                                    ModuleExportName::Ident(id) => id.sym.to_string(),
                                    ModuleExportName::Str(s) => s.value.to_string(),
                                };
                                let exported_name = named
                                    .exported
                                    .as_ref()
                                    .map(|e| match e {
                                        ModuleExportName::Ident(id) => id.sym.to_string(),
                                        ModuleExportName::Str(s) => s.value.to_string(),
                                    })
                                    .unwrap_or_else(|| orig_name.clone());

                                // The layout was added to imported_layouts in the first pass
                                if let Some(layout) = imported_layouts.get(&exported_name) {
                                    exports
                                        .layouts
                                        .insert(exported_name.clone(), layout.clone());
                                }
                                if let Some(when) = imported_when.get(&exported_name) {
                                    exports.when.insert(exported_name, when.clone());
                                }
                            }
                        }
                    } else {
                        // Local re-export: export { localVar }
                        for spec in &named_export.specifiers {
                            if let ExportSpecifier::Named(named) = spec {
                                let orig_name = match &named.orig {
                                    ModuleExportName::Ident(id) => id.sym.to_string(),
                                    ModuleExportName::Str(s) => s.value.to_string(),
                                };
                                let exported_name = named
                                    .exported
                                    .as_ref()
                                    .map(|e| match e {
                                        ModuleExportName::Ident(id) => id.sym.to_string(),
                                        ModuleExportName::Str(s) => s.value.to_string(),
                                    })
                                    .unwrap_or_else(|| orig_name.clone());

                                // Check if it's an imported layout
                                if let Some(layout) = imported_layouts.get(&orig_name) {
                                    exports
                                        .layouts
                                        .insert(exported_name.clone(), layout.clone());
                                } else if let Some(var_expr) = variables.get(&orig_name) {
                                    // It's a local variable being exported
                                    if let Ok(layout) = self.eval_layout_node(
                                        var_expr,
                                        &variables,
                                        &imported_layouts,
                                    ) {
                                        exports.layouts.insert(exported_name.clone(), layout);
                                    } else if let Ok(when) =
                                        self.eval_when(var_expr, &variables, &imported_when)
                                    {
                                        exports.when.insert(exported_name.clone(), when);
                                    } else if let Ok(arr) = self.eval_string_array(var_expr) {
                                        exports.string_arrays.insert(exported_name.clone(), arr);
                                    } else if let Ok(rules) = self.eval_rules(var_expr, &variables)
                                    {
                                        exports.rules.insert(exported_name.clone(), rules);
                                    } else if let Ok(mirror) =
                                        self.eval_mirror(var_expr, &variables)
                                    {
                                        exports.mirrors.insert(exported_name.clone(), mirror);
                                    }
                                }

                                if let Some(when) = imported_when.get(&orig_name) {
                                    exports.when.insert(exported_name.clone(), when.clone());
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        self.module_exports_cache
            .borrow_mut()
            .insert(path.to_path_buf(), exports.clone());
        Ok(exports)
    }

    #[allow(clippy::too_many_arguments)]
    fn eval_define_config(
        &self,
        expr: &Expr,
        variables: &HashMap<String, &Expr>,
        imported_layouts: &HashMap<String, LayoutNode>,
        imported_when: &HashMap<String, HashMap<String, WhenRequirement>>,
        imported_string_arrays: &HashMap<String, Vec<String>>,
        imported_rules: &HashMap<String, RulesConfig>,
        imported_mirrors: &HashMap<String, Vec<MirrorConfig>>,
    ) -> Result<ConfigIR, ParseError> {
        if let Expr::Call(call) = expr {
            if let Callee::Expr(callee_expr) = &call.callee {
                if let Expr::Ident(ident) = &**callee_expr {
                    let fn_name = ident.sym.as_ref();
                    if fn_name == "defineConfig" {
                        if let Some(arg) = call.args.first() {
                            if let Expr::Call(inner_call) = &*arg.expr {
                                if let Callee::Expr(inner_callee) = &inner_call.callee {
                                    if let Expr::Ident(inner_ident) = &**inner_callee {
                                        if inner_ident.sym.as_ref() == "nextjsPreset" {
                                            return self.eval_nextjs_preset(
                                                inner_call,
                                                variables,
                                                imported_layouts,
                                            );
                                        }
                                    }
                                }
                            }
                            return self.eval_config_object(
                                &arg.expr,
                                variables,
                                imported_layouts,
                                imported_when,
                                imported_string_arrays,
                                imported_rules,
                                imported_mirrors,
                            );
                        }
                    } else if fn_name == "nextjsPreset" {
                        return self.eval_nextjs_preset(call, variables, imported_layouts);
                    }
                }
            }
        }
        let loc = self.get_expr_location(expr);
        Err(ParseError::UnsupportedExpression {
            line: loc.0,
            col: loc.1,
            message: "Expected defineConfig({...}) or nextjsPreset(...)".to_string(),
        })
    }

    fn eval_nextjs_preset(
        &self,
        call: &CallExpr,
        _variables: &HashMap<String, &Expr>,
        _imported_layouts: &HashMap<String, LayoutNode>,
    ) -> Result<ConfigIR, ParseError> {
        let mut route_case = CaseStyle::Kebab;
        // let mut require_tests = false;

        if let Some(arg) = call.args.first() {
            if let Ok(obj) = self.expect_object(&arg.expr) {
                for prop in &obj.props {
                    if let PropOrSpread::Prop(prop) = prop {
                        if let Prop::KeyValue(kv) = &**prop {
                            let key = self.get_prop_name(&kv.key)?;
                            if key == "routeCase" {
                                route_case = self.eval_case_style(&kv.value)?;
                            }
                        }
                    }
                }
            }
        }

        // Basic Next.js App Router structure
        let mut app_children = HashMap::new();
        app_children.insert(
            "layout.tsx".to_string(),
            LayoutNode::File {
                pattern: None,
                optional: false,
                required: true,
                case: None,
            },
        );
        app_children.insert(
            "page.tsx".to_string(),
            LayoutNode::File {
                pattern: None,
                optional: true,
                required: false,
                case: None,
            },
        );

        let layout = LayoutNode::Dir {
            children: {
                let mut root = HashMap::new();
                root.insert(
                    "app".to_string(),
                    LayoutNode::Dir {
                        children: {
                            let mut app = HashMap::new();
                            app.insert(
                                "$routes".to_string(),
                                LayoutNode::Recursive {
                                    max_depth: 10,
                                    child: Box::new(LayoutNode::Param {
                                        name: "route".to_string(),
                                        case: route_case,
                                        child: Box::new(LayoutNode::Dir {
                                            children: app_children,
                                            optional: false,
                                            required: false,
                                            strict: false,
                                            max_depth: None,
                                        }),
                                    }),
                                },
                            );
                            app
                        },
                        optional: false,
                        required: true,
                        strict: false,
                        max_depth: None,
                    },
                );
                root
            },
            optional: false,
            required: false,
            strict: false,
            max_depth: None,
        };

        Ok(ConfigIR::new(layout))
    }

    #[allow(clippy::too_many_arguments)]
    fn eval_config_object(
        &self,
        expr: &Expr,
        variables: &HashMap<String, &Expr>,
        imported_layouts: &HashMap<String, LayoutNode>,
        imported_when: &HashMap<String, HashMap<String, WhenRequirement>>,
        imported_string_arrays: &HashMap<String, Vec<String>>,
        imported_rules: &HashMap<String, RulesConfig>,
        imported_mirrors: &HashMap<String, Vec<MirrorConfig>>,
    ) -> Result<ConfigIR, ParseError> {
        let obj = self.expect_object(expr)?;

        let mut mode = Mode::default();
        let mut layout = None;
        let mut rules = RulesConfig::default();
        let mut boundaries = None;
        let mut deps = None;
        let mut ignore = Vec::new();
        let mut use_gitignore = true;
        let mut workspaces = Vec::new();
        let mut dependencies = HashMap::new();
        let mut mirror = Vec::new();
        let mut when = HashMap::new();
        let mut extends = None;

        for prop in &obj.props {
            if let PropOrSpread::Prop(prop) = prop {
                match &**prop {
                    Prop::KeyValue(kv) => {
                        let key = self.get_prop_name(&kv.key)?;
                        match key.as_str() {
                            "mode" => mode = self.eval_mode(&kv.value)?,
                            "layout" => {
                                layout = Some(self.eval_layout_node(
                                    &kv.value,
                                    variables,
                                    imported_layouts,
                                )?)
                            }
                            "rules" => {
                                rules = self.eval_rules_with_imports(
                                    &kv.value,
                                    variables,
                                    imported_rules,
                                )?
                            }
                            "boundaries" => {
                                boundaries = Some(self.eval_boundaries(&kv.value, variables)?)
                            }
                            "deps" => deps = Some(self.eval_deps(&kv.value, variables)?),
                            "ignore" => {
                                ignore = self.eval_string_array_with_imports(
                                    &kv.value,
                                    variables,
                                    imported_string_arrays,
                                )?
                            }
                            "useGitignore" => use_gitignore = self.eval_bool(&kv.value)?,
                            "workspaces" => {
                                workspaces = self.eval_string_array_with_imports(
                                    &kv.value,
                                    variables,
                                    imported_string_arrays,
                                )?
                            }
                            "dependencies" => dependencies = self.eval_dependencies(&kv.value)?,
                            "mirror" => {
                                mirror = self.eval_mirror_with_imports(
                                    &kv.value,
                                    variables,
                                    imported_mirrors,
                                )?
                            }
                            "when" => when = self.eval_when(&kv.value, variables, imported_when)?,
                            "extends" => extends = Some(self.expect_string(&kv.value)?),
                            _ => {}
                        }
                    }
                    Prop::Shorthand(ident) => {
                        let key = ident.sym.to_string();
                        match key.as_str() {
                            "layout" => {
                                if let Some(layout_node) = imported_layouts.get(&key) {
                                    layout = Some(layout_node.clone());
                                } else if let Some(var_expr) = variables.get(&key) {
                                    layout = Some(self.eval_layout_node(
                                        var_expr,
                                        variables,
                                        imported_layouts,
                                    )?);
                                } else {
                                    let loc = self.get_expr_location(expr);
                                    return Err(ParseError::UnsupportedExpression {
                                        line: loc.0,
                                        col: loc.1,
                                        message: format!(
                                            "Unknown shorthand value for layout: {}",
                                            key
                                        ),
                                    });
                                }
                            }
                            "rules" => {
                                if let Some(r) = imported_rules.get(&key) {
                                    rules = r.clone();
                                } else if let Some(var_expr) = variables.get(&key) {
                                    rules = self.eval_rules(var_expr, variables)?;
                                }
                            }
                            "boundaries" => {
                                if let Some(var_expr) = variables.get(&key) {
                                    boundaries = Some(self.eval_boundaries(var_expr, variables)?);
                                }
                            }
                            "deps" => {
                                if let Some(var_expr) = variables.get(&key) {
                                    deps = Some(self.eval_deps(var_expr, variables)?);
                                }
                            }
                            "mirror" => {
                                if let Some(m) = imported_mirrors.get(&key) {
                                    mirror = m.clone();
                                } else if let Some(var_expr) = variables.get(&key) {
                                    mirror = self.eval_mirror(var_expr, variables)?;
                                }
                            }
                            "when" => {
                                if let Some(w) = imported_when.get(&key) {
                                    when = w.clone();
                                } else if let Some(var_expr) = variables.get(&key) {
                                    when = self.eval_when(var_expr, variables, imported_when)?;
                                }
                            }
                            "ignore" => {
                                if let Some(arr) = imported_string_arrays.get(&key) {
                                    ignore = arr.clone();
                                } else if let Some(var_expr) = variables.get(&key) {
                                    ignore = self.eval_string_array(var_expr)?;
                                }
                            }
                            "workspaces" => {
                                if let Some(arr) = imported_string_arrays.get(&key) {
                                    workspaces = arr.clone();
                                } else if let Some(var_expr) = variables.get(&key) {
                                    workspaces = self.eval_string_array(var_expr)?;
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(ConfigIR {
            mode,
            layout,
            rules,
            boundaries,
            deps,
            ignore,
            use_gitignore,
            workspaces,
            dependencies,
            mirror,
            when,
            extends,
        })
    }

    fn eval_mode(&self, expr: &Expr) -> Result<Mode, ParseError> {
        let s = self.expect_string(expr)?;
        match s.as_str() {
            "strict" => Ok(Mode::Strict),
            "warn" => Ok(Mode::Warn),
            _ => Err(ParseError::InvalidValue {
                field: "mode".to_string(),
                message: format!("expected 'strict' or 'warn', got '{}'", s),
            }),
        }
    }

    fn eval_layout_node(
        &self,
        expr: &Expr,
        variables: &HashMap<String, &Expr>,
        imported_layouts: &HashMap<String, LayoutNode>,
    ) -> Result<LayoutNode, ParseError> {
        if let Expr::Ident(ident) = expr {
            let var_name = ident.sym.as_ref();
            if let Some(layout) = imported_layouts.get(var_name) {
                return Ok(layout.clone());
            }
            if let Some(var_expr) = variables.get(var_name) {
                return self.eval_layout_node(var_expr, variables, imported_layouts);
            }
            let loc = self.get_expr_location(expr);
            return Err(ParseError::UnsupportedExpression {
                line: loc.0,
                col: loc.1,
                message: format!("Unknown variable: {}", var_name),
            });
        }
        if let Expr::Call(call) = expr {
            if let Callee::Expr(callee_expr) = &call.callee {
                if let Expr::Ident(ident) = &**callee_expr {
                    let fn_name = ident.sym.as_ref();
                    return match fn_name {
                        "dir" | "directory" => self.eval_dir(call, variables, imported_layouts),
                        "file" => self.eval_file(call),
                        "opt" | "optional" => self.eval_opt(call, variables, imported_layouts),
                        "required" => self.eval_required(call, variables, imported_layouts),
                        "param" => self.eval_param(call, variables, imported_layouts),
                        "many" => self.eval_many(call, variables, imported_layouts),
                        "recursive" => self.eval_recursive(call, variables, imported_layouts),
                        "either" => self.eval_either(call, variables, imported_layouts),
                        _ => {
                            let loc = self.get_expr_location(expr);
                            Err(ParseError::UnsupportedExpression {
                                line: loc.0,
                                col: loc.1,
                                message: format!("Unknown DSL function: {}", fn_name),
                            })
                        }
                    };
                }
            }
        }
        let loc = self.get_expr_location(expr);
        Err(ParseError::UnsupportedExpression {
            line: loc.0,
            col: loc.1,
            message:
                "Expected DSL function call (directory, file, optional, required, param, many)"
                    .to_string(),
        })
    }

    fn eval_dir(
        &self,
        call: &CallExpr,
        variables: &HashMap<String, &Expr>,
        imported_layouts: &HashMap<String, LayoutNode>,
    ) -> Result<LayoutNode, ParseError> {
        let mut children = HashMap::new();
        let mut strict = false;
        let mut max_depth = None;

        if let Some(arg) = call.args.first() {
            let obj = self.expect_object(&arg.expr)?;
            for prop in &obj.props {
                if let PropOrSpread::Prop(prop) = prop {
                    match &**prop {
                        Prop::KeyValue(kv) => {
                            let key = self.get_prop_name(&kv.key)?;
                            let value =
                                self.eval_layout_node(&kv.value, variables, imported_layouts)?;
                            children.insert(key, value);
                        }
                        Prop::Shorthand(ident) => {
                            let key = ident.sym.to_string();
                            if let Some(layout) = imported_layouts.get(&key) {
                                children.insert(key, layout.clone());
                            } else if let Some(var_expr) = variables.get(&key) {
                                let value =
                                    self.eval_layout_node(var_expr, variables, imported_layouts)?;
                                children.insert(key, value);
                            } else {
                                let loc = self.get_expr_location(&arg.expr);
                                return Err(ParseError::UnsupportedExpression {
                                    line: loc.0,
                                    col: loc.1,
                                    message: format!("Unknown shorthand variable: {}", key),
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if call.args.len() >= 2 {
            if let Ok(opts) = self.expect_object(&call.args[1].expr) {
                for prop in &opts.props {
                    if let PropOrSpread::Prop(prop) = prop {
                        if let Prop::KeyValue(kv) = &**prop {
                            let key = self.get_prop_name(&kv.key)?;
                            match key.as_str() {
                                "strict" => strict = self.expect_bool(&kv.value)?,
                                "maxDepth" => {
                                    max_depth = Some(self.expect_number(&kv.value)? as usize)
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        Ok(LayoutNode::Dir {
            children,
            optional: false,
            required: false,
            strict,
            max_depth,
        })
    }

    fn eval_file(&self, call: &CallExpr) -> Result<LayoutNode, ParseError> {
        if let Some(arg) = call.args.first() {
            if let Ok(s) = self.expect_string(&arg.expr) {
                return Ok(LayoutNode::File {
                    pattern: Some(s),
                    optional: false,
                    required: false,
                    case: None,
                });
            }
            if let Ok(obj) = self.expect_object(&arg.expr) {
                let mut pattern = None;
                let mut case = None;
                for prop in &obj.props {
                    if let PropOrSpread::Prop(prop) = prop {
                        if let Prop::KeyValue(kv) = &**prop {
                            let key = self.get_prop_name(&kv.key)?;
                            match key.as_str() {
                                "pattern" => pattern = Some(self.expect_string(&kv.value)?),
                                "case" => case = Some(self.eval_case_style(&kv.value)?),
                                _ => {}
                            }
                        }
                    }
                }
                return Ok(LayoutNode::File {
                    pattern,
                    optional: false,
                    required: false,
                    case,
                });
            }
        }
        Ok(LayoutNode::File {
            pattern: None,
            optional: false,
            required: false,
            case: None,
        })
    }

    fn eval_opt(
        &self,
        call: &CallExpr,
        variables: &HashMap<String, &Expr>,
        imported_layouts: &HashMap<String, LayoutNode>,
    ) -> Result<LayoutNode, ParseError> {
        if let Some(arg) = call.args.first() {
            let mut node = self.eval_layout_node(&arg.expr, variables, imported_layouts)?;
            match &mut node {
                LayoutNode::Dir { optional, .. } => *optional = true,
                LayoutNode::File { optional, .. } => *optional = true,
                _ => {}
            }
            return Ok(node);
        }
        Err(ParseError::MissingField(
            "optional() requires an argument".to_string(),
        ))
    }

    fn eval_required(
        &self,
        call: &CallExpr,
        variables: &HashMap<String, &Expr>,
        imported_layouts: &HashMap<String, LayoutNode>,
    ) -> Result<LayoutNode, ParseError> {
        if let Some(arg) = call.args.first() {
            let mut node = self.eval_layout_node(&arg.expr, variables, imported_layouts)?;
            match &mut node {
                LayoutNode::Dir { required, .. } => *required = true,
                LayoutNode::File { required, .. } => *required = true,
                _ => {}
            }
            return Ok(node);
        }
        Err(ParseError::MissingField(
            "required() requires an argument".to_string(),
        ))
    }

    fn eval_param(
        &self,
        call: &CallExpr,
        variables: &HashMap<String, &Expr>,
        imported_layouts: &HashMap<String, LayoutNode>,
    ) -> Result<LayoutNode, ParseError> {
        if call.args.len() < 2 {
            return Err(ParseError::MissingField(
                "param() requires options and child arguments".to_string(),
            ));
        }

        let opts_obj = self.expect_object(&call.args[0].expr)?;
        let mut case = CaseStyle::Any;
        let mut name = String::new();

        for prop in &opts_obj.props {
            if let PropOrSpread::Prop(prop) = prop {
                if let Prop::KeyValue(kv) = &**prop {
                    let key = self.get_prop_name(&kv.key)?;
                    match key.as_str() {
                        "case" => case = self.eval_case_style(&kv.value)?,
                        "name" => name = self.expect_string(&kv.value)?,
                        _ => {}
                    }
                }
            }
        }

        if name.is_empty() {
            let opts_str = self.expect_string(&call.args[0].expr).ok();
            name = opts_str.unwrap_or_else(|| "$param".to_string());
        }

        let child = self.eval_layout_node(&call.args[1].expr, variables, imported_layouts)?;

        Ok(LayoutNode::Param {
            name,
            case,
            child: Box::new(child),
        })
    }

    fn eval_many(
        &self,
        call: &CallExpr,
        variables: &HashMap<String, &Expr>,
        imported_layouts: &HashMap<String, LayoutNode>,
    ) -> Result<LayoutNode, ParseError> {
        let (case, max, child_idx) = if call.args.len() >= 2 {
            if let Ok(obj) = self.expect_object(&call.args[0].expr) {
                let mut case = None;
                let mut max = None;
                for prop in &obj.props {
                    if let PropOrSpread::Prop(prop) = prop {
                        if let Prop::KeyValue(kv) = &**prop {
                            let key = self.get_prop_name(&kv.key)?;
                            match key.as_str() {
                                "case" => case = Some(self.eval_case_style(&kv.value)?),
                                "max" => max = Some(self.expect_number(&kv.value)? as usize),
                                _ => {}
                            }
                        }
                    }
                }
                (case, max, 1)
            } else {
                (None, None, 0)
            }
        } else {
            (None, None, 0)
        };

        if call.args.len() <= child_idx {
            return Err(ParseError::MissingField(
                "many() requires a child argument".to_string(),
            ));
        }

        let child =
            self.eval_layout_node(&call.args[child_idx].expr, variables, imported_layouts)?;

        Ok(LayoutNode::Many {
            case,
            child: Box::new(child),
            max,
        })
    }

    fn eval_recursive(
        &self,
        call: &CallExpr,
        variables: &HashMap<String, &Expr>,
        imported_layouts: &HashMap<String, LayoutNode>,
    ) -> Result<LayoutNode, ParseError> {
        let (max_depth, child_idx) = if call.args.len() >= 2 {
            if let Ok(obj) = self.expect_object(&call.args[0].expr) {
                let mut max_depth = 10usize;
                for prop in &obj.props {
                    if let PropOrSpread::Prop(prop) = prop {
                        if let Prop::KeyValue(kv) = &**prop {
                            let key = self.get_prop_name(&kv.key)?;
                            if key == "maxDepth" {
                                max_depth = self.expect_number(&kv.value)? as usize;
                            }
                        }
                    }
                }
                (max_depth, 1)
            } else {
                (10, 0)
            }
        } else {
            (10, 0)
        };

        if call.args.len() <= child_idx {
            return Err(ParseError::MissingField(
                "recursive() requires a child argument".to_string(),
            ));
        }

        let child =
            self.eval_layout_node(&call.args[child_idx].expr, variables, imported_layouts)?;

        Ok(LayoutNode::Recursive {
            max_depth,
            child: Box::new(child),
        })
    }

    fn eval_either(
        &self,
        call: &CallExpr,
        variables: &HashMap<String, &Expr>,
        imported_layouts: &HashMap<String, LayoutNode>,
    ) -> Result<LayoutNode, ParseError> {
        if call.args.is_empty() {
            return Err(ParseError::MissingField(
                "either() requires at least one variant".to_string(),
            ));
        }

        let mut variants = Vec::new();
        for arg in &call.args {
            let variant = self.eval_layout_node(&arg.expr, variables, imported_layouts)?;
            variants.push(variant);
        }

        Ok(LayoutNode::Either { variants })
    }

    fn eval_case_style(&self, expr: &Expr) -> Result<CaseStyle, ParseError> {
        let s = self.expect_string(expr)?;
        match s.as_str() {
            "kebab" => Ok(CaseStyle::Kebab),
            "snake" => Ok(CaseStyle::Snake),
            "camel" => Ok(CaseStyle::Camel),
            "pascal" => Ok(CaseStyle::Pascal),
            "any" => Ok(CaseStyle::Any),
            _ => Err(ParseError::InvalidValue {
                field: "case".to_string(),
                message: format!(
                    "expected 'kebab', 'snake', 'camel', 'pascal', or 'any', got '{}'",
                    s
                ),
            }),
        }
    }

    fn eval_rules(
        &self,
        expr: &Expr,
        variables: &HashMap<String, &Expr>,
    ) -> Result<RulesConfig, ParseError> {
        if let Expr::Ident(ident) = expr {
            let name = ident.sym.as_ref();
            if let Some(var_expr) = variables.get(name) {
                return self.eval_rules(var_expr, variables);
            }
        }
        let obj = self.expect_object(expr)?;
        let mut rules = RulesConfig::default();

        for prop in &obj.props {
            if let PropOrSpread::Prop(prop) = prop {
                if let Prop::KeyValue(kv) = &**prop {
                    let key = self.get_prop_name(&kv.key)?;
                    match key.as_str() {
                        "forbidPaths" => rules.forbid_paths = self.eval_string_array(&kv.value)?,
                        "forbidNames" => rules.forbid_names = self.eval_string_array(&kv.value)?,
                        "ignorePaths" => rules.ignore_paths = self.eval_string_array(&kv.value)?,
                        _ => {}
                    }
                }
            }
        }

        Ok(rules)
    }

    fn eval_boundaries(
        &self,
        expr: &Expr,
        variables: &HashMap<String, &Expr>,
    ) -> Result<BoundariesConfig, ParseError> {
        if let Expr::Ident(ident) = expr {
            let name = ident.sym.as_ref();
            if let Some(var_expr) = variables.get(name) {
                return self.eval_boundaries(var_expr, variables);
            }
        }
        let obj = self.expect_object(expr)?;
        let mut modules = String::new();
        let mut public_api = String::new();
        let mut forbid_deep_imports = false;

        for prop in &obj.props {
            if let PropOrSpread::Prop(prop) = prop {
                if let Prop::KeyValue(kv) = &**prop {
                    let key = self.get_prop_name(&kv.key)?;
                    match key.as_str() {
                        "modules" => modules = self.expect_string(&kv.value)?,
                        "publicApi" => public_api = self.expect_string(&kv.value)?,
                        "forbidDeepImports" => forbid_deep_imports = self.expect_bool(&kv.value)?,
                        _ => {}
                    }
                }
            }
        }

        Ok(BoundariesConfig {
            modules,
            public_api,
            forbid_deep_imports,
        })
    }

    fn eval_deps(
        &self,
        expr: &Expr,
        variables: &HashMap<String, &Expr>,
    ) -> Result<DepsConfig, ParseError> {
        if let Expr::Ident(ident) = expr {
            let name = ident.sym.as_ref();
            if let Some(var_expr) = variables.get(name) {
                return self.eval_deps(var_expr, variables);
            }
        }
        let obj = self.expect_object(expr)?;
        let mut deps = DepsConfig::default();

        for prop in &obj.props {
            if let PropOrSpread::Prop(prop) = prop {
                if let Prop::KeyValue(kv) = &**prop {
                    let key = self.get_prop_name(&kv.key)?;
                    if key == "allow" {
                        deps.allow = self.eval_deps_allow(&kv.value)?;
                    }
                }
            }
        }

        Ok(deps)
    }

    fn eval_deps_allow(&self, expr: &Expr) -> Result<Vec<DepsAllowRule>, ParseError> {
        let arr = self.expect_array(expr)?;
        let mut rules = Vec::new();

        for elem in arr.elems.iter().flatten() {
            let obj = self.expect_object(&elem.expr)?;
            let mut from = String::new();
            let mut to = Vec::new();

            for prop in &obj.props {
                if let PropOrSpread::Prop(prop) = prop {
                    if let Prop::KeyValue(kv) = &**prop {
                        let key = self.get_prop_name(&kv.key)?;
                        match key.as_str() {
                            "from" => from = self.expect_string(&kv.value)?,
                            "to" => to = self.eval_string_array(&kv.value)?,
                            _ => {}
                        }
                    }
                }
            }

            rules.push(DepsAllowRule { from, to });
        }

        Ok(rules)
    }

    fn eval_string_array(&self, expr: &Expr) -> Result<Vec<String>, ParseError> {
        let arr = self.expect_array(expr)?;
        let mut result = Vec::new();
        for elem in arr.elems.iter().flatten() {
            result.push(self.expect_string(&elem.expr)?);
        }
        Ok(result)
    }

    fn eval_string_array_with_imports(
        &self,
        expr: &Expr,
        variables: &HashMap<String, &Expr>,
        imported_string_arrays: &HashMap<String, Vec<String>>,
    ) -> Result<Vec<String>, ParseError> {
        // Check if it's an identifier that might be imported
        if let Expr::Ident(ident) = expr {
            let name = ident.sym.as_ref();
            // First check imported string arrays
            if let Some(arr) = imported_string_arrays.get(name) {
                return Ok(arr.clone());
            }
            // Then check local variables
            if let Some(var_expr) = variables.get(name) {
                return self.eval_string_array(var_expr);
            }
        }
        // Fall back to direct evaluation
        self.eval_string_array(expr)
    }

    fn eval_rules_with_imports(
        &self,
        expr: &Expr,
        variables: &HashMap<String, &Expr>,
        imported_rules: &HashMap<String, RulesConfig>,
    ) -> Result<RulesConfig, ParseError> {
        // Check if it's an identifier that might be imported
        if let Expr::Ident(ident) = expr {
            let name = ident.sym.as_ref();
            // First check imported rules
            if let Some(rules) = imported_rules.get(name) {
                return Ok(rules.clone());
            }
            // Then check local variables
            if let Some(var_expr) = variables.get(name) {
                return self.eval_rules(var_expr, variables);
            }
        }
        // Fall back to direct evaluation
        self.eval_rules(expr, variables)
    }

    fn eval_mirror_with_imports(
        &self,
        expr: &Expr,
        variables: &HashMap<String, &Expr>,
        imported_mirrors: &HashMap<String, Vec<MirrorConfig>>,
    ) -> Result<Vec<MirrorConfig>, ParseError> {
        // Check if it's an identifier that might be imported
        if let Expr::Ident(ident) = expr {
            let name = ident.sym.as_ref();
            // First check imported mirrors
            if let Some(mirror) = imported_mirrors.get(name) {
                return Ok(mirror.clone());
            }
            // Then check local variables
            if let Some(var_expr) = variables.get(name) {
                return self.eval_mirror(var_expr, variables);
            }
        }
        // Fall back to direct evaluation
        self.eval_mirror(expr, variables)
    }

    fn eval_dependencies(&self, expr: &Expr) -> Result<HashMap<String, String>, ParseError> {
        let obj = self.expect_object(expr)?;
        let mut deps = HashMap::new();

        for prop in &obj.props {
            if let PropOrSpread::Prop(prop) = prop {
                if let Prop::KeyValue(kv) = &**prop {
                    let key = self.get_prop_name(&kv.key)?;
                    let value = self.expect_string(&kv.value)?;
                    deps.insert(key, value);
                }
            }
        }

        Ok(deps)
    }

    fn eval_mirror(
        &self,
        expr: &Expr,
        variables: &HashMap<String, &Expr>,
    ) -> Result<Vec<MirrorConfig>, ParseError> {
        if let Expr::Ident(ident) = expr {
            let name = ident.sym.as_ref();
            if let Some(var_expr) = variables.get(name) {
                return self.eval_mirror(var_expr, variables);
            }
        }
        let arr = self.expect_array(expr)?;
        let mut mirrors = Vec::new();

        for elem in arr.elems.iter().flatten() {
            let obj = self.expect_object(&elem.expr)?;
            let mut source = String::new();
            let mut target = String::new();
            let mut pattern = String::new();

            for prop in &obj.props {
                if let PropOrSpread::Prop(prop) = prop {
                    if let Prop::KeyValue(kv) = &**prop {
                        let key = self.get_prop_name(&kv.key)?;
                        match key.as_str() {
                            "source" => source = self.expect_string(&kv.value)?,
                            "target" => target = self.expect_string(&kv.value)?,
                            "pattern" => pattern = self.expect_string(&kv.value)?,
                            _ => {}
                        }
                    }
                }
            }

            mirrors.push(MirrorConfig {
                source,
                target,
                pattern,
            });
        }

        Ok(mirrors)
    }

    fn eval_when(
        &self,
        expr: &Expr,
        variables: &HashMap<String, &Expr>,
        imported_when: &HashMap<String, HashMap<String, WhenRequirement>>,
    ) -> Result<HashMap<String, WhenRequirement>, ParseError> {
        if let Expr::Ident(ident) = expr {
            let name = ident.sym.as_ref();
            if let Some(w) = imported_when.get(name) {
                return Ok(w.clone());
            }
            if let Some(var_expr) = variables.get(name) {
                return self.eval_when(var_expr, variables, imported_when);
            }
        }

        let obj = self.expect_object(expr)?;
        let mut when = HashMap::new();

        for prop in &obj.props {
            if let PropOrSpread::Prop(prop) = prop {
                if let Prop::KeyValue(kv) = &**prop {
                    let key = self.get_prop_name(&kv.key)?;
                    let req_obj = self.expect_object(&kv.value)?;
                    let mut requires = Vec::new();

                    for req_prop in &req_obj.props {
                        if let PropOrSpread::Prop(req_prop) = req_prop {
                            if let Prop::KeyValue(req_kv) = &**req_prop {
                                let req_key = self.get_prop_name(&req_kv.key)?;
                                if req_key == "requires" {
                                    requires = self.eval_string_array(&req_kv.value)?;
                                }
                            }
                        }
                    }

                    when.insert(key, WhenRequirement { requires });
                }
            }
        }

        Ok(when)
    }

    fn expect_object<'a>(&self, expr: &'a Expr) -> Result<&'a ObjectLit, ParseError> {
        if let Expr::Object(obj) = expr {
            return Ok(obj);
        }
        let loc = self.get_expr_location(expr);
        Err(ParseError::UnsupportedExpression {
            line: loc.0,
            col: loc.1,
            message: "Expected object literal".to_string(),
        })
    }

    fn expect_array<'a>(&self, expr: &'a Expr) -> Result<&'a ArrayLit, ParseError> {
        if let Expr::Array(arr) = expr {
            return Ok(arr);
        }
        let loc = self.get_expr_location(expr);
        Err(ParseError::UnsupportedExpression {
            line: loc.0,
            col: loc.1,
            message: "Expected array literal".to_string(),
        })
    }

    fn expect_string(&self, expr: &Expr) -> Result<String, ParseError> {
        if let Expr::Lit(Lit::Str(s)) = expr {
            return Ok(s.value.to_string());
        }
        if let Expr::Tpl(tpl) = expr {
            if tpl.exprs.is_empty() && tpl.quasis.len() == 1 {
                return Ok(tpl.quasis[0].raw.to_string());
            }
        }
        let loc = self.get_expr_location(expr);
        Err(ParseError::UnsupportedExpression {
            line: loc.0,
            col: loc.1,
            message: "Expected string literal".to_string(),
        })
    }

    fn expect_bool(&self, expr: &Expr) -> Result<bool, ParseError> {
        if let Expr::Lit(Lit::Bool(b)) = expr {
            return Ok(b.value);
        }
        let loc = self.get_expr_location(expr);
        Err(ParseError::UnsupportedExpression {
            line: loc.0,
            col: loc.1,
            message: "Expected boolean literal".to_string(),
        })
    }

    fn eval_bool(&self, expr: &Expr) -> Result<bool, ParseError> {
        self.expect_bool(expr)
    }

    fn expect_number(&self, expr: &Expr) -> Result<f64, ParseError> {
        if let Expr::Lit(Lit::Num(n)) = expr {
            return Ok(n.value);
        }
        let loc = self.get_expr_location(expr);
        Err(ParseError::UnsupportedExpression {
            line: loc.0,
            col: loc.1,
            message: "Expected number literal".to_string(),
        })
    }

    fn get_prop_name(&self, key: &PropName) -> Result<String, ParseError> {
        match key {
            PropName::Ident(i) => Ok(i.sym.to_string()),
            PropName::Str(s) => Ok(s.value.to_string()),
            PropName::Computed(c) => {
                if let Expr::Lit(Lit::Str(s)) = &*c.expr {
                    return Ok(s.value.to_string());
                }
                Ok("$computed".to_string())
            }
            _ => Ok("$unknown".to_string()),
        }
    }

    fn get_expr_location(&self, expr: &Expr) -> (usize, usize) {
        let span = match expr {
            Expr::Call(c) => c.span,
            Expr::Object(o) => o.span,
            Expr::Lit(l) => match l {
                Lit::Str(s) => s.span,
                Lit::Bool(b) => b.span,
                Lit::Num(n) => n.span,
                _ => return (0, 0),
            },
            _ => return (0, 0),
        };
        if span.lo.0 == 0 {
            return (0, 0);
        }
        let loc = self.source_map.lookup_char_pos(span.lo);
        (loc.line, loc.col_display)
    }
}

impl Default for ConfigParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_simple_config() {
        let parser = ConfigParser::new();
        let config = r#"
import { defineConfig, dir, file } from "repo-lint";

export default defineConfig({
    mode: "strict",
    layout: dir({
        src: dir({
            "index.ts": file(),
        }),
    }),
});
"#;
        let result = parser.parse_string(config, "test.ts");
        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.mode, Mode::Strict);
    }

    #[test]
    fn test_parse_with_rules() {
        let parser = ConfigParser::new();
        let config = r#"
import { defineConfig, dir, file } from "repo-lint";

export default defineConfig({
    layout: dir({}),
    rules: {
        forbidPaths: ["**/utils/**", "**/*.bak"],
        forbidNames: ["temp", "test"],
    },
});
"#;
        let result = parser.parse_string(config, "test.ts");
        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.rules.forbid_paths.len(), 2);
        assert_eq!(ir.rules.forbid_names.len(), 2);
    }

    #[test]
    fn test_parse_with_param() {
        let parser = ConfigParser::new();
        let config = r#"
import { defineConfig, dir, file, param } from "repo-lint";

export default defineConfig({
    layout: dir({
        modules: dir({
            $module: param({ case: "kebab" }, dir({
                "index.ts": file(),
            })),
        }),
    }),
});
"#;
        let result = parser.parse_string(config, "test.ts");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_missing_layout() {
        let parser = ConfigParser::new();
        let config = r#"
export default defineConfig({
    mode: "strict",
});
"#;
        let result = parser.parse_string(config, "test.ts");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_with_boundaries() {
        let parser = ConfigParser::new();
        let config = r#"
import { defineConfig, dir } from "repo-lint";

export default defineConfig({
    layout: dir({}),
    boundaries: {
        modules: "src/services/*",
        publicApi: "src/services/*/api/index.ts",
        forbidDeepImports: true,
    },
});
"#;
        let result = parser.parse_string(config, "test.ts");
        assert!(result.is_ok());
        let ir = result.unwrap();
        assert!(ir.boundaries.is_some());
        let boundaries = ir.boundaries.unwrap();
        assert_eq!(boundaries.modules, "src/services/*");
        assert!(boundaries.forbid_deep_imports);
    }

    #[test]
    fn test_parse_with_deps() {
        let parser = ConfigParser::new();
        let config = r#"
import { defineConfig, dir } from "repo-lint";

export default defineConfig({
    layout: dir({}),
    deps: {
        allow: [
            { from: "src/api/**", to: ["src/domain/**"] },
            { from: "src/domain/**", to: [] },
        ],
    },
});
"#;
        let result = parser.parse_string(config, "test.ts");
        assert!(result.is_ok());
        let ir = result.unwrap();
        assert!(ir.deps.is_some());
        let deps = ir.deps.unwrap();
        assert_eq!(deps.allow.len(), 2);
    }

    #[test]
    fn test_parse_with_optional() {
        let parser = ConfigParser::new();
        let config = r#"
import { defineConfig, dir, file, opt } from "repo-lint";

export default defineConfig({
    layout: dir({
        src: dir({}),
        tests: opt(dir({})),
        "README.md": opt(file()),
    }),
});
"#;
        let result = parser.parse_string(config, "test.ts");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_with_many() {
        let parser = ConfigParser::new();
        let config = r#"
import { defineConfig, dir, file, many } from "repo-lint";

export default defineConfig({
    layout: dir({
        routes: dir({
            $route: many({ case: "kebab" }, dir({
                "index.ts": file(),
            })),
        }),
    }),
});
"#;
        let result = parser.parse_string(config, "test.ts");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_warn_mode() {
        let parser = ConfigParser::new();
        let config = r#"
import { defineConfig, dir } from "repo-lint";

export default defineConfig({
    mode: "warn",
    layout: dir({}),
});
"#;
        let result = parser.parse_string(config, "test.ts");
        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.mode, Mode::Warn);
    }

    #[test]
    fn test_parse_invalid_mode() {
        let parser = ConfigParser::new();
        let config = r#"
import { defineConfig, dir } from "repo-lint";

export default defineConfig({
    mode: "invalid",
    layout: dir({}),
});
"#;
        let result = parser.parse_string(config, "test.ts");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_file_with_pattern() {
        let parser = ConfigParser::new();
        let config = r#"
import { defineConfig, dir, file } from "repo-lint";

export default defineConfig({
    layout: dir({
        $any: file("*.ts"),
    }),
});
"#;
        let result = parser.parse_string(config, "test.ts");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parser_default() {
        let parser = ConfigParser::default();
        let config = r#"
import { defineConfig, dir } from "repo-lint";
export default defineConfig({ layout: dir({}) });
"#;
        let result = parser.parse_string(config, "test.ts");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_with_recursive() {
        let parser = ConfigParser::new();
        let config = r#"
import { defineConfig, dir, file, param, recursive } from "repo-lint";

export default defineConfig({
    layout: dir({
        app: dir({
            $routes: recursive(param({ case: "kebab" }, dir({
                "page.tsx": file(),
            }))),
        }),
    }),
});
"#;
        let result = parser.parse_string(config, "test.ts");
        assert!(result.is_ok());
        let ir = result.unwrap();
        if let Some(LayoutNode::Dir { children, .. }) = &ir.layout {
            if let Some(LayoutNode::Dir {
                children: app_children,
                ..
            }) = children.get("app")
            {
                assert!(app_children.contains_key("$routes"));
                if let Some(LayoutNode::Recursive { max_depth, .. }) = app_children.get("$routes") {
                    assert_eq!(*max_depth, 10);
                }
            }
        }
    }

    #[test]
    fn test_parse_with_recursive_custom_depth() {
        let parser = ConfigParser::new();
        let config = r#"
import { defineConfig, dir, file, param, recursive } from "repo-lint";

export default defineConfig({
    layout: dir({
        app: dir({
            $routes: recursive({ maxDepth: 5 }, param({ case: "kebab" }, dir({
                "page.tsx": file(),
            }))),
        }),
    }),
});
"#;
        let result = parser.parse_string(config, "test.ts");
        assert!(result.is_ok());
        let ir = result.unwrap();
        if let Some(LayoutNode::Dir { children, .. }) = &ir.layout {
            if let Some(LayoutNode::Dir {
                children: app_children,
                ..
            }) = children.get("app")
            {
                if let Some(LayoutNode::Recursive { max_depth, .. }) = app_children.get("$routes") {
                    assert_eq!(*max_depth, 5);
                }
            }
        }
    }

    #[test]
    fn test_parse_with_either() {
        let parser = ConfigParser::new();
        let config = r#"
import { defineConfig, dir, file, either } from "repo-lint";

export default defineConfig({
    layout: dir({
        routes: dir({
            $segment: either(
                file("page.tsx"),
                dir({ "index.ts": file() }),
            ),
        }),
    }),
});
"#;
        let result = parser.parse_string(config, "test.ts");
        assert!(result.is_ok());
        let ir = result.unwrap();
        if let Some(LayoutNode::Dir { children, .. }) = &ir.layout {
            if let Some(LayoutNode::Dir {
                children: routes_children,
                ..
            }) = children.get("routes")
            {
                if let Some(LayoutNode::Either { variants }) = routes_children.get("$segment") {
                    assert_eq!(variants.len(), 2);
                }
            }
        }
    }

    #[test]
    fn test_parse_with_ignore_config() {
        let parser = ConfigParser::new();
        let config = r#"
import { defineConfig, dir } from "repo-lint";

export default defineConfig({
    layout: dir({}),
    ignore: [".git", "node_modules", ".next"],
    useGitignore: false,
});
"#;
        let result = parser.parse_string(config, "test.ts");
        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.ignore, vec![".git", "node_modules", ".next"]);
        assert!(!ir.use_gitignore);
    }

    #[test]
    fn test_parse_with_ignore_paths_rule() {
        let parser = ConfigParser::new();
        let config = r#"
import { defineConfig, dir } from "repo-lint";

export default defineConfig({
    layout: dir({}),
    rules: {
        ignorePaths: ["**/node_modules/**", "**/.turbo/**"],
        forbidPaths: ["**/utils/**"],
    },
});
"#;
        let result = parser.parse_string(config, "test.ts");
        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(
            ir.rules.ignore_paths,
            vec!["**/node_modules/**", "**/.turbo/**"]
        );
        assert_eq!(ir.rules.forbid_paths, vec!["**/utils/**"]);
    }

    #[test]
    fn test_imported_layout_can_reference_imported_nested_directory_helpers() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        let config_dir = root.join("packages/config/repo-lint");
        fs::create_dir_all(&config_dir).unwrap();

        // A nested module that exports a layout.
        fs::write(
            config_dir.join("feature.ts"),
            r#"
import { directory, file } from "repo-lint";

export const featureModule = directory({
  asset: directory({
    components: directory({
      "index.ts": file(),
    }),
  }),
});
"#,
        )
        .unwrap();

        // The module we import from: it imports featureModule and uses it inside param().
        fs::write(
            config_dir.join("nextjs.ts"),
            r#"
import { directory, param } from "repo-lint";
import { featureModule } from "./feature";

export const nextjsAppLayout = directory({
  features: directory({
    $domain: param({ case: "kebab" }, featureModule),
  }),
});
"#,
        )
        .unwrap();

        // Root config importing the exported layout.
        fs::write(
            root.join("repo-lint.config.ts"),
            r#"
import { defineConfig } from "repo-lint";
import { nextjsAppLayout } from "./packages/config/repo-lint/nextjs";

export default defineConfig({
  layout: nextjsAppLayout,
});
"#,
        )
        .unwrap();

        let parser = ConfigParser::new();
        let ir = parser
            .parse_file(&root.join("repo-lint.config.ts"))
            .unwrap();
        let layout = ir.layout.unwrap();

        // Assert: features/$domain(param)/asset/components exists (i.e., nested directory helpers were evaluated).
        let LayoutNode::Dir { children, .. } = layout else {
            panic!("expected root dir");
        };
        let LayoutNode::Dir { children, .. } = children.get("features").unwrap() else {
            panic!("expected features dir");
        };
        let LayoutNode::Param { child, .. } = children.get("$domain").unwrap() else {
            panic!("expected $domain param");
        };
        let LayoutNode::Dir { children, .. } = child.as_ref() else {
            panic!("expected param child dir");
        };
        let LayoutNode::Dir { children, .. } = children.get("asset").unwrap() else {
            panic!("expected asset dir");
        };
        assert!(children.contains_key("components"));
    }

    #[test]
    fn test_when_can_be_imported_from_module() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        fs::write(
            root.join("conds.ts"),
            r#"
export const elysiaWhenConditions = {
  "src/index.ts": { requires: ["src/app.ts"] },
};
"#,
        )
        .unwrap();

        fs::write(
            root.join("repo-lint.config.ts"),
            r#"
import { defineConfig, dir } from "repo-lint";
import { elysiaWhenConditions } from "./conds";

export default defineConfig({
  layout: dir({}),
  when: elysiaWhenConditions,
});
"#,
        )
        .unwrap();

        let parser = ConfigParser::new();
        let ir = parser
            .parse_file(&root.join("repo-lint.config.ts"))
            .unwrap();
        assert_eq!(
            ir.when.get("src/index.ts").unwrap().requires.as_slice(),
            &["src/app.ts"]
        );
    }

    #[test]
    fn test_optional_imported_nested_layout_is_traversed_in_matching() {
        use crate::engine::layout_trie::{LayoutMatcher, MatchResult};

        let temp = TempDir::new().unwrap();
        let root = temp.path();

        fs::write(
            root.join("shared.ts"),
            r#"
import { directory, file, many, optional, param } from "repo-lint";

export const nested = directory({
  $file: many(file("*.ts")),
});

export const layout = directory({
  $domain: param({ case: "kebab" }, directory({
    subdir: optional(nested),
  })),
});
"#,
        )
        .unwrap();

        fs::write(
            root.join("repo-lint.config.ts"),
            r#"
import { defineConfig } from "repo-lint";
import { layout } from "./shared";

export default defineConfig({
  layout,
});
"#,
        )
        .unwrap();

        let parser = ConfigParser::new();
        let ir = parser
            .parse_file(&root.join("repo-lint.config.ts"))
            .unwrap();
        let layout = ir.layout.unwrap();

        let matcher = LayoutMatcher::new(Some(layout));
        let result = matcher.match_path(Path::new("domain-name/subdir/file.ts"));
        assert!(matches!(
            result,
            MatchResult::AllowedMany { .. }
                | MatchResult::AllowedParam { .. }
                | MatchResult::Allowed
        ));
    }

    #[test]
    fn test_directory_children_support_shorthand_values() {
        let parser = ConfigParser::new();
        let config = r#"
import { defineConfig, directory, file } from "repo-lint";

const leaf = directory({
  "index.ts": file(),
});

export default defineConfig({
  layout: directory({
    src: leaf,
  }),
});
"#;

        let ir = parser.parse_string(config, "test.ts").unwrap();
        let Some(LayoutNode::Dir { children, .. }) = ir.layout else {
            panic!("expected layout");
        };
        let LayoutNode::Dir { children, .. } = children.get("src").unwrap() else {
            panic!("expected src to be a dir");
        };
        assert!(children.contains_key("index.ts"));
    }

    #[test]
    fn test_imported_layout_references_local_const_in_same_file() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        let config_dir = root.join("packages/config/repo-lint");
        fs::create_dir_all(&config_dir).unwrap();

        // The key difference: featureModule is NOT exported, just a local const
        fs::write(
            config_dir.join("nextjs.ts"),
            r#"
import { directory, param, file } from "repo-lint";

// LOCAL const (not exported!)
const featureModule = directory({
  components: directory({
    "index.ts": file(),
  }),
});

export const nextjsAppLayout = directory({
  features: directory({
    $domain: param({ case: "kebab" }, featureModule),
  }),
});
"#,
        )
        .unwrap();

        // Root config importing the exported layout
        fs::write(
            root.join("repo-lint.config.ts"),
            r#"
import { defineConfig } from "repo-lint";
import { nextjsAppLayout } from "./packages/config/repo-lint/nextjs";

export default defineConfig({
  layout: nextjsAppLayout,
});
"#,
        )
        .unwrap();

        let parser = ConfigParser::new();
        let ir = parser
            .parse_file(&root.join("repo-lint.config.ts"))
            .unwrap();
        let layout = ir.layout.unwrap();

        // Assert: features/$domain(param)/components exists
        let LayoutNode::Dir { children, .. } = layout else {
            panic!("expected root dir");
        };
        let LayoutNode::Dir { children, .. } = children.get("features").unwrap() else {
            panic!("expected features dir");
        };
        let LayoutNode::Param { child, .. } = children.get("$domain").unwrap() else {
            panic!("expected $domain param");
        };
        let LayoutNode::Dir { children, .. } = child.as_ref() else {
            panic!("expected param child dir (featureModule)");
        };
        assert!(
            children.contains_key("components"),
            "featureModule const was not resolved - components dir missing"
        );
    }

    #[test]
    fn test_imported_layout_with_deeply_nested_local_consts() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        let config_dir = root.join("packages/config");
        fs::create_dir_all(&config_dir).unwrap();

        // Multiple local consts referencing each other
        fs::write(
            config_dir.join("layouts.ts"),
            r#"
import { directory, param, file, many } from "repo-lint";

// These are all LOCAL consts (not exported)
const fileNode = file("*.ts");

const componentsDir = directory({
  "index.ts": file(),
  $component: many(fileNode),
});

const featureModule = directory({
  components: componentsDir,
  hooks: directory({
    "index.ts": file(),
  }),
});

// Only this is exported
export const appLayout = directory({
  features: directory({
    $domain: param({ case: "kebab" }, featureModule),
  }),
});
"#,
        )
        .unwrap();

        fs::write(
            root.join("repo-lint.config.ts"),
            r#"
import { defineConfig } from "repo-lint";
import { appLayout } from "./packages/config/layouts";

export default defineConfig({
  layout: appLayout,
});
"#,
        )
        .unwrap();

        let parser = ConfigParser::new();
        let ir = parser
            .parse_file(&root.join("repo-lint.config.ts"))
            .unwrap();
        let layout = ir.layout.unwrap();

        // Verify the deeply nested structure was resolved
        let LayoutNode::Dir { children, .. } = layout else {
            panic!("expected root dir");
        };
        let LayoutNode::Dir { children, .. } = children.get("features").unwrap() else {
            panic!("expected features dir");
        };
        let LayoutNode::Param { child, .. } = children.get("$domain").unwrap() else {
            panic!("expected $domain param");
        };
        let LayoutNode::Dir { children, .. } = child.as_ref() else {
            panic!("expected featureModule dir");
        };

        // Check componentsDir was resolved
        let LayoutNode::Dir {
            children: comp_children,
            ..
        } = children.get("components").unwrap()
        else {
            panic!("expected components dir (componentsDir const)");
        };
        assert!(
            comp_children.contains_key("index.ts"),
            "components should have index.ts"
        );
        assert!(
            comp_children.contains_key("$component"),
            "components should have $component (fileNode const)"
        );

        // Check hooks was resolved
        assert!(
            children.contains_key("hooks"),
            "featureModule should have hooks"
        );
    }

    #[test]
    fn test_imported_layout_from_reexporting_module() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        let config_dir = root.join("packages/config");
        fs::create_dir_all(&config_dir).unwrap();

        // Original module with local const
        fs::write(
            config_dir.join("base.ts"),
            r#"
import { directory, file, param } from "repo-lint";

const routeContent = directory({
  "page.tsx": file(),
  "layout.tsx": file(),
});

export const baseLayout = directory({
  app: directory({
    $route: param({ case: "kebab" }, routeContent),
  }),
});
"#,
        )
        .unwrap();

        // Re-exporting module
        fs::write(
            config_dir.join("index.ts"),
            r#"
export { baseLayout } from "./base";
"#,
        )
        .unwrap();

        fs::write(
            root.join("repo-lint.config.ts"),
            r#"
import { defineConfig } from "repo-lint";
import { baseLayout } from "./packages/config";

export default defineConfig({
  layout: baseLayout,
});
"#,
        )
        .unwrap();

        let parser = ConfigParser::new();
        let ir = parser
            .parse_file(&root.join("repo-lint.config.ts"))
            .unwrap();
        let layout = ir.layout.unwrap();

        // Verify structure
        let LayoutNode::Dir { children, .. } = layout else {
            panic!("expected root dir");
        };
        let LayoutNode::Dir { children, .. } = children.get("app").unwrap() else {
            panic!("expected app dir");
        };
        let LayoutNode::Param { child, .. } = children.get("$route").unwrap() else {
            panic!("expected $route param");
        };
        let LayoutNode::Dir { children, .. } = child.as_ref() else {
            panic!("expected routeContent dir");
        };
        assert!(
            children.contains_key("page.tsx"),
            "routeContent const should be resolved"
        );
        assert!(
            children.contains_key("layout.tsx"),
            "routeContent const should be resolved"
        );
    }

    #[test]
    fn test_imported_layout_with_brace_expansion_in_file() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        let config_dir = root.join("packages/config");
        fs::create_dir_all(&config_dir).unwrap();

        fs::write(
            config_dir.join("nextjs.ts"),
            r#"
import { directory, param, file, many, optional } from "repo-lint";

const featureModule = directory({
  components: optional(directory({
    $f: many(file('*.{ts,tsx}')),
  })),
});

export const layout = directory({
  features: directory({
    $domain: param({ case: 'kebab' }, featureModule),
  }),
});
"#,
        )
        .unwrap();

        fs::write(
            root.join("repo-lint.config.ts"),
            r#"
import { defineConfig } from "repo-lint";
import { layout } from "./packages/config/nextjs";

export default defineConfig({ layout });
"#,
        )
        .unwrap();

        let parser = ConfigParser::new();
        let ir = parser
            .parse_file(&root.join("repo-lint.config.ts"))
            .unwrap();
        let layout_node = ir.layout.unwrap();

        // Navigate to features/$domain/components/$f
        let LayoutNode::Dir { children, .. } = layout_node else {
            panic!("expected root dir");
        };
        let LayoutNode::Dir { children, .. } = children.get("features").unwrap() else {
            panic!("expected features dir");
        };
        let LayoutNode::Param { child, .. } = children.get("$domain").unwrap() else {
            panic!("expected $domain param");
        };
        let LayoutNode::Dir { children, .. } = child.as_ref() else {
            panic!("expected featureModule dir");
        };
        let LayoutNode::Dir {
            children, optional, ..
        } = children.get("components").unwrap()
        else {
            panic!("expected components dir");
        };
        assert!(*optional, "components should be optional");

        let LayoutNode::Many { child, .. } = children.get("$f").unwrap() else {
            panic!("expected $f many");
        };
        let LayoutNode::File { pattern, .. } = child.as_ref() else {
            panic!("expected file node");
        };
        assert_eq!(
            pattern.as_deref(),
            Some("*.{ts,tsx}"),
            "file pattern should be preserved"
        );
    }

    #[test]
    fn test_imported_layout_with_file_object_syntax() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        let config_dir = root.join("packages/config");
        fs::create_dir_all(&config_dir).unwrap();

        fs::write(
            config_dir.join("nextjs.ts"),
            r#"
import { directory, param, file, many } from "repo-lint";

const featureModule = directory({
  components: directory({
    $f: many(file({ pattern: '*.ts', case: 'kebab' })),
  }),
});

export const layout = directory({
  features: directory({
    $domain: param({ case: 'kebab' }, featureModule),
  }),
});
"#,
        )
        .unwrap();

        fs::write(
            root.join("repo-lint.config.ts"),
            r#"
import { defineConfig } from "repo-lint";
import { layout } from "./packages/config/nextjs";

export default defineConfig({ layout });
"#,
        )
        .unwrap();

        let parser = ConfigParser::new();
        let ir = parser
            .parse_file(&root.join("repo-lint.config.ts"))
            .unwrap();
        let layout_node = ir.layout.unwrap();

        // Navigate to features/$domain/components/$f
        let LayoutNode::Dir { children, .. } = layout_node else {
            panic!("expected root dir");
        };
        let LayoutNode::Dir { children, .. } = children.get("features").unwrap() else {
            panic!("expected features dir");
        };
        let LayoutNode::Param { child, .. } = children.get("$domain").unwrap() else {
            panic!("expected $domain param");
        };
        let LayoutNode::Dir { children, .. } = child.as_ref() else {
            panic!("expected featureModule dir");
        };
        let LayoutNode::Dir { children, .. } = children.get("components").unwrap() else {
            panic!("expected components dir");
        };

        let LayoutNode::Many { child, .. } = children.get("$f").unwrap() else {
            panic!("expected $f many");
        };
        let LayoutNode::File { pattern, case, .. } = child.as_ref() else {
            panic!("expected file node");
        };
        assert_eq!(
            pattern.as_deref(),
            Some("*.ts"),
            "file pattern should be preserved"
        );
        assert!(
            matches!(case, Some(CaseStyle::Kebab)),
            "file case should be kebab"
        );
    }

    #[test]
    fn test_resolve_workspace_package_ignores_node_modules() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        fs::write(
            root.join("package.json"),
            r#"{"workspaces":["packages/*"]}"#,
        )
        .unwrap();

        fs::create_dir_all(root.join("packages/node_modules/@acme/pkg/src")).unwrap();
        fs::write(
            root.join("packages/node_modules/@acme/pkg/package.json"),
            r#"{"name":"@acme/pkg"}"#,
        )
        .unwrap();
        fs::write(
            root.join("packages/node_modules/@acme/pkg/src/index.ts"),
            "",
        )
        .unwrap();

        let parser = ConfigParser::new();
        let resolved = parser.resolve_workspace_package(root, "@acme/pkg");
        assert!(resolved.is_none());
    }
}
