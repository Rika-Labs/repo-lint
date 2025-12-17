use std::collections::HashMap;
use std::path::Path;
use swc_common::sync::Lrc;
use swc_common::{FileName, SourceMap, Spanned};
use swc_ecma_ast::*;
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};
use thiserror::Error;

use super::ir::{
    BoundariesConfig, CaseStyle, ConfigIR, DepsAllowRule, DepsConfig, LayoutNode, Mode, RulesConfig,
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

pub struct ConfigParser {
    source_map: Lrc<SourceMap>,
}

impl ConfigParser {
    pub fn new() -> Self {
        Self {
            source_map: Lrc::new(SourceMap::default()),
        }
    }

    pub fn parse_file(&self, path: &Path) -> Result<ConfigIR, ParseError> {
        let content = std::fs::read_to_string(path)?;
        self.parse_string(&content, path.to_string_lossy().as_ref())
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

        self.extract_config(&module)
    }

    fn extract_config(&self, module: &Module) -> Result<ConfigIR, ParseError> {
        for item in &module.body {
            if let ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(export)) = item {
                return self.eval_define_config(&export.expr);
            }
        }
        Err(ParseError::MissingField(
            "export default defineConfig(...)".to_string(),
        ))
    }

    fn eval_define_config(&self, expr: &Expr) -> Result<ConfigIR, ParseError> {
        if let Expr::Call(call) = expr {
            if let Callee::Expr(callee_expr) = &call.callee {
                if let Expr::Ident(ident) = &**callee_expr {
                    if ident.sym.as_ref() == "defineConfig" {
                        if let Some(arg) = call.args.first() {
                            return self.eval_config_object(&arg.expr);
                        }
                    }
                }
            }
        }
        let loc = self.get_expr_location(expr);
        Err(ParseError::UnsupportedExpression {
            line: loc.0,
            col: loc.1,
            message: "Expected defineConfig({...})".to_string(),
        })
    }

    fn eval_config_object(&self, expr: &Expr) -> Result<ConfigIR, ParseError> {
        let obj = self.expect_object(expr)?;

        let mut mode = Mode::default();
        let mut layout = None;
        let mut rules = RulesConfig::default();
        let mut boundaries = None;
        let mut deps = None;
        let mut ignore = Vec::new();
        let mut use_gitignore = true;

        for prop in &obj.props {
            if let PropOrSpread::Prop(prop) = prop {
                if let Prop::KeyValue(kv) = &**prop {
                    let key = self.get_prop_name(&kv.key)?;
                    match key.as_str() {
                        "mode" => mode = self.eval_mode(&kv.value)?,
                        "layout" => layout = Some(self.eval_layout_node(&kv.value)?),
                        "rules" => rules = self.eval_rules(&kv.value)?,
                        "boundaries" => boundaries = Some(self.eval_boundaries(&kv.value)?),
                        "deps" => deps = Some(self.eval_deps(&kv.value)?),
                        "ignore" => ignore = self.eval_string_array(&kv.value)?,
                        "useGitignore" => use_gitignore = self.eval_bool(&kv.value)?,
                        _ => {}
                    }
                }
            }
        }

        let layout = layout.ok_or(ParseError::MissingField("layout".to_string()))?;

        Ok(ConfigIR {
            mode,
            layout,
            rules,
            boundaries,
            deps,
            ignore,
            use_gitignore,
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

    fn eval_layout_node(&self, expr: &Expr) -> Result<LayoutNode, ParseError> {
        if let Expr::Call(call) = expr {
            if let Callee::Expr(callee_expr) = &call.callee {
                if let Expr::Ident(ident) = &**callee_expr {
                    let fn_name = ident.sym.as_ref();
                    return match fn_name {
                        "dir" => self.eval_dir(call),
                        "file" => self.eval_file(call),
                        "opt" => self.eval_opt(call),
                        "param" => self.eval_param(call),
                        "many" => self.eval_many(call),
                        "recursive" => self.eval_recursive(call),
                        "either" => self.eval_either(call),
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
            message: "Expected DSL function call (dir, file, opt, param, many)".to_string(),
        })
    }

    fn eval_dir(&self, call: &CallExpr) -> Result<LayoutNode, ParseError> {
        let mut children = HashMap::new();
        if let Some(arg) = call.args.first() {
            let obj = self.expect_object(&arg.expr)?;
            for prop in &obj.props {
                if let PropOrSpread::Prop(prop) = prop {
                    if let Prop::KeyValue(kv) = &**prop {
                        let key = self.get_prop_name(&kv.key)?;
                        let value = self.eval_layout_node(&kv.value)?;
                        children.insert(key, value);
                    }
                }
            }
        }
        Ok(LayoutNode::Dir {
            children,
            optional: false,
        })
    }

    fn eval_file(&self, call: &CallExpr) -> Result<LayoutNode, ParseError> {
        let pattern = if let Some(arg) = call.args.first() {
            Some(self.expect_string(&arg.expr)?)
        } else {
            None
        };
        Ok(LayoutNode::File {
            pattern,
            optional: false,
        })
    }

    fn eval_opt(&self, call: &CallExpr) -> Result<LayoutNode, ParseError> {
        if let Some(arg) = call.args.first() {
            let mut node = self.eval_layout_node(&arg.expr)?;
            match &mut node {
                LayoutNode::Dir { optional, .. } => *optional = true,
                LayoutNode::File { optional, .. } => *optional = true,
                _ => {}
            }
            return Ok(node);
        }
        Err(ParseError::MissingField(
            "opt() requires an argument".to_string(),
        ))
    }

    fn eval_param(&self, call: &CallExpr) -> Result<LayoutNode, ParseError> {
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

        let child = self.eval_layout_node(&call.args[1].expr)?;

        Ok(LayoutNode::Param {
            name,
            case,
            child: Box::new(child),
        })
    }

    fn eval_many(&self, call: &CallExpr) -> Result<LayoutNode, ParseError> {
        let (case, child_idx) = if call.args.len() >= 2 {
            if let Ok(obj) = self.expect_object(&call.args[0].expr) {
                let mut case = None;
                for prop in &obj.props {
                    if let PropOrSpread::Prop(prop) = prop {
                        if let Prop::KeyValue(kv) = &**prop {
                            let key = self.get_prop_name(&kv.key)?;
                            if key == "case" {
                                case = Some(self.eval_case_style(&kv.value)?);
                            }
                        }
                    }
                }
                (case, 1)
            } else {
                (None, 0)
            }
        } else {
            (None, 0)
        };

        if call.args.len() <= child_idx {
            return Err(ParseError::MissingField(
                "many() requires a child argument".to_string(),
            ));
        }

        let child = self.eval_layout_node(&call.args[child_idx].expr)?;

        Ok(LayoutNode::Many {
            case,
            child: Box::new(child),
        })
    }

    fn eval_recursive(&self, call: &CallExpr) -> Result<LayoutNode, ParseError> {
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

        let child = self.eval_layout_node(&call.args[child_idx].expr)?;

        Ok(LayoutNode::Recursive {
            max_depth,
            child: Box::new(child),
        })
    }

    fn eval_either(&self, call: &CallExpr) -> Result<LayoutNode, ParseError> {
        if call.args.is_empty() {
            return Err(ParseError::MissingField(
                "either() requires at least one variant".to_string(),
            ));
        }

        let mut variants = Vec::new();
        for arg in &call.args {
            let variant = self.eval_layout_node(&arg.expr)?;
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
        assert!(result.is_err());
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
        if let LayoutNode::Dir { children, .. } = &ir.layout {
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
        if let LayoutNode::Dir { children, .. } = &ir.layout {
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
        if let LayoutNode::Dir { children, .. } = &ir.layout {
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
