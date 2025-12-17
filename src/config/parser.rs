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
    layout_cache: RefCell<HashMap<PathBuf, HashMap<String, LayoutNode>>>,
}

impl ConfigParser {
    pub fn new() -> Self {
        Self {
            source_map: Lrc::new(SourceMap::default()),
            layout_cache: RefCell::new(HashMap::new()),
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
        if specifier.starts_with("@/") {
            let mut root = dir;
            let mut current = Some(dir);
            while let Some(d) = current {
                if d.join("repo-lint.config.ts").exists() {
                    root = d;
                }
                current = d.parent();
            }
            let mut path = root.join(&specifier[2..]);
            if !path.exists() && path.extension().is_none() {
                path.set_extension("ts");
            }
            if path.exists() {
                return Some(path);
            }
        }

        // Try relative path
        if specifier.starts_with('.') {
            let mut path = dir.join(specifier);
            if !path.exists() && path.extension().is_none() {
                path.set_extension("ts");
            }
            if path.exists() {
                return Some(path);
            }
            // Try index.ts if it's a directory
            let dir_path = dir.join(specifier);
            if dir_path.is_dir() {
                let index_path = dir_path.join("index.ts");
                if index_path.exists() {
                    return Some(index_path);
                }
            }
        }

        // Try node_modules (basic)
        let mut current_dir = Some(dir);
        while let Some(d) = current_dir {
            let node_modules = d.join("node_modules");
            if node_modules.exists() {
                let pkg_path = node_modules.join(specifier);
                let mut path = pkg_path.clone();
                if !path.exists() && path.extension().is_none() {
                    path.set_extension("ts");
                }
                if path.exists() {
                    return Some(path);
                }
                // Try package.json main if it's a directory
                if pkg_path.is_dir() {
                    let config_path = pkg_path.join("repo-lint.config.ts");
                    if config_path.exists() {
                        return Some(config_path);
                    }
                }
            }
            current_dir = d.parent();
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
                                match spec {
                                    ImportSpecifier::Named(named) => {
                                        let local = named.local.sym.to_string();
                                        let imported = named
                                            .imported
                                            .as_ref()
                                            .map(|i| match i {
                                                ModuleExportName::Ident(id) => id.sym.to_string(),
                                                ModuleExportName::Str(s) => s.value.to_string(),
                                            })
                                            .unwrap_or_else(|| local.clone());

                                        if let Some(layout) = exports.get(&imported) {
                                            imported_layouts.insert(local, layout.clone());
                                        }
                                    }
                                    _ => {}
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
                return self.eval_define_config(&export.expr, &variables, &imported_layouts);
            }
        }
        Err(ParseError::MissingField(
            "export default defineConfig(...)".to_string(),
        ))
    }

    fn parse_module_exports(&self, path: &Path) -> Result<HashMap<String, LayoutNode>, ParseError> {
        if let Some(cached) = self.layout_cache.borrow().get(path) {
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

        let mut exports = HashMap::new();
        let mut variables = HashMap::new();
        let imported_layouts = HashMap::new();

        // Basic extraction similar to extract_config but looking for exports
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
                                    let name = ident.sym.to_string();
                                    variables.insert(name.clone(), init.as_ref());
                                    if let Ok(layout) =
                                        self.eval_layout_node(init, &variables, &imported_layouts)
                                    {
                                        exports.insert(name, layout);
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        self.layout_cache
            .borrow_mut()
            .insert(path.to_path_buf(), exports.clone());
        Ok(exports)
    }

    fn eval_define_config(
        &self,
        expr: &Expr,
        variables: &HashMap<String, &Expr>,
        imported_layouts: &HashMap<String, LayoutNode>,
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
                            return self.eval_config_object(&arg.expr, variables, imported_layouts);
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
                            match key.as_str() {
                                "routeCase" => route_case = self.eval_case_style(&kv.value)?,
                                // "requireTests" => require_tests = self.expect_bool(&kv.value)?,
                                _ => {}
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

    fn eval_config_object(
        &self,
        expr: &Expr,
        variables: &HashMap<String, &Expr>,
        imported_layouts: &HashMap<String, LayoutNode>,
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
                if let Prop::KeyValue(kv) = &**prop {
                    let key = self.get_prop_name(&kv.key)?;
                    match key.as_str() {
                        "mode" => mode = self.eval_mode(&kv.value)?,
                        "layout" => {
                            layout =
                                Some(self.eval_layout_node(&kv.value, variables, imported_layouts)?)
                        }
                        "rules" => rules = self.eval_rules(&kv.value)?,
                        "boundaries" => boundaries = Some(self.eval_boundaries(&kv.value)?),
                        "deps" => deps = Some(self.eval_deps(&kv.value)?),
                        "ignore" => ignore = self.eval_string_array(&kv.value)?,
                        "useGitignore" => use_gitignore = self.eval_bool(&kv.value)?,
                        "workspaces" => workspaces = self.eval_string_array(&kv.value)?,
                        "dependencies" => dependencies = self.eval_dependencies(&kv.value)?,
                        "mirror" => mirror = self.eval_mirror(&kv.value)?,
                        "when" => when = self.eval_when(&kv.value)?,
                        "extends" => extends = Some(self.expect_string(&kv.value)?),
                        _ => {}
                    }
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
                    if let Prop::KeyValue(kv) = &**prop {
                        let key = self.get_prop_name(&kv.key)?;
                        let value = self.eval_layout_node(&kv.value, variables, imported_layouts)?;
                        children.insert(key, value);
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

        let child = self.eval_layout_node(&call.args[child_idx].expr, variables, imported_layouts)?;

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

        let child = self.eval_layout_node(&call.args[child_idx].expr, variables, imported_layouts)?;

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

    fn eval_rules(&self, expr: &Expr) -> Result<RulesConfig, ParseError> {
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

    fn eval_boundaries(&self, expr: &Expr) -> Result<BoundariesConfig, ParseError> {
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

    fn eval_deps(&self, expr: &Expr) -> Result<DepsConfig, ParseError> {
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

    fn eval_mirror(&self, expr: &Expr) -> Result<Vec<MirrorConfig>, ParseError> {
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

    fn eval_when(&self, expr: &Expr) -> Result<HashMap<String, WhenRequirement>, ParseError> {
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
}
