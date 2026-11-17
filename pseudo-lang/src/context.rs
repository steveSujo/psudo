use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::lexer::Lexer;
use crate::parser::{Expression, Parser, Statment};
use crate::resolver::Resolver;

// Inferring &T / &mut T / move modes from AST usage lets `.psudo` stay free of lifetime
// annotations while still lowering cleanly to borrow-checked Rust.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParamBorrowMode {
    ReadShared,    // &T (read-only in body)
    MutateInPlace, // &mut T (reassigned or mutated in body)
    OwnedMove,     // T (moved into collection/return/assignment)
}

impl ParamBorrowMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ParamBorrowMode::ReadShared => "&",
            ParamBorrowMode::MutateInPlace => "&mut ",
            ParamBorrowMode::OwnedMove => "move ",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataflowStep {
    pub source_vars: Vec<String>,
    pub target: String,
    pub op_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolContextDigest {
    pub name: String,
    pub kind: String, // "fn" | "class" | "script"
    pub params: Vec<(String, ParamBorrowMode)>,
    pub is_stub: bool,
    pub stub_directive: Option<String>,
    pub calls: Vec<String>,
    pub io_effects: Vec<String>, // "pure" | "fs:read" | "fs:write" | "clock"
    pub dataflow: Vec<DataflowStep>,
    pub return_sources: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleContextDigest {
    pub module_path: String,
    pub estimated_tokens: usize,
    pub diagnostics: Vec<String>,
    pub symbols: Vec<SymbolContextDigest>,
    pub call_graph_edges: Vec<(String, String)>,
    pub zettels: Vec<String>,
}

pub fn analyze_psudo_source(
    source: &str,
    module_path: &str,
    zettel_path: Option<&Path>,
) -> Result<ModuleContextDigest, String> {
    let mut lexer = Lexer::new(source.to_string());
    let tokens = match lexer.scan_tokens() {
        Ok(t) => t,
        Err(e) => return Err(format!("Lexer error: {}", e)),
    };

    let mut parser = Parser::new(tokens);
    let statements = match parser.parse() {
        Ok(stmts) => stmts,
        Err(err) => return Err(format!("Parse error: {}", err)),
    };

    let mut diagnostics = Vec::new();
    let resolver = Resolver::new();
    let res_result = resolver.resolve(&statements);
    for err in res_result.errors {
        diagnostics.push(format!(
            "[line {}, col {}] {}",
            err.line, err.column, err.message
        ));
    }

    let mut symbols = Vec::new();
    let mut call_graph_edges = Vec::new();

    for stmt in &statements {
        match stmt {
            Statment::ExpressionStatement(Expression::Function(params, body)) => {
                let fn_name = "anonymous".to_string();
                let fn_params = params.clone();
                let digest = analyze_function(&fn_name, &fn_params, body, &mut call_graph_edges);
                symbols.push(digest);
            }
            Statment::ExpressionStatement(Expression::Assign(fn_name, expr))
                if matches!(**expr, Expression::Function(..)) =>
            {
                if let Expression::Function(params, body) = &**expr {
                    let fn_params = params.clone();
                    let digest = analyze_function(fn_name, &fn_params, body, &mut call_graph_edges);
                    symbols.push(digest);
                }
            }
            Statment::ClassStatement(name_tok, methods, _super_tok) => {
                let class_name = name_tok.text.clone();
                let mut class_calls = Vec::new();
                for m in methods {
                    if let Statment::ExpressionStatement(Expression::Function(m_params, m_body)) = m
                    {
                        let m_name = m_params
                            .first()
                            .cloned()
                            .unwrap_or_else(|| "method".to_string());
                        let full_name = format!("{}::{}", class_name, m_name);
                        let m_args = if m_params.len() > 1 {
                            m_params[1..].to_vec()
                        } else {
                            vec![]
                        };
                        let m_digest =
                            analyze_function(&full_name, &m_args, m_body, &mut call_graph_edges);
                        symbols.push(m_digest);
                        class_calls.push(m_name);
                    }
                }
                symbols.push(SymbolContextDigest {
                    name: class_name,
                    kind: "class".to_string(),
                    params: vec![],
                    is_stub: false,
                    stub_directive: None,
                    calls: class_calls,
                    io_effects: vec!["pure".to_string()],
                    dataflow: vec![],
                    return_sources: vec![],
                });
            }
            _ => {}
        }
    }

    // Attach matching Zettel notes if present
    let mut zettels = Vec::new();
    let z_candidates = [
        zettel_path.map(|p| p.to_path_buf()),
        Some(Path::new(".pseudo/state/zettels.json").to_path_buf()),
        Some(Path::new("../.pseudo/state/zettels.json").to_path_buf()),
    ];
    for cand in z_candidates.into_iter().flatten() {
        if cand.exists() {
            if let Ok(content) = std::fs::read_to_string(&cand) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(records) = val.get("records").and_then(|r| r.as_object()) {
                        for (_, rec) in records {
                            let is_tombstone = rec
                                .get("is_tombstone")
                                .and_then(|b| b.as_bool())
                                .unwrap_or(false);
                            if is_tombstone {
                                continue;
                            }
                            let scope = rec.get("scope").and_then(|s| s.as_str()).unwrap_or("");
                            let fact = rec.get("fact").and_then(|f| f.as_str()).unwrap_or("");
                            let kind = rec.get("kind").and_then(|k| k.as_str()).unwrap_or("");
                            if symbols
                                .iter()
                                .any(|s| s.name.contains(scope) || scope.contains(&s.name))
                                || scope == "workspace"
                            {
                                zettels.push(format!("[{}] {}: {}", kind, scope, fact));
                                if zettels.len() >= 5 {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            break;
        }
    }

    let estimated_tokens = source.len() / 4 + 10;

    Ok(ModuleContextDigest {
        module_path: module_path.to_string(),
        estimated_tokens,
        diagnostics,
        symbols,
        call_graph_edges,
        zettels,
    })
}

fn analyze_function(
    name: &str,
    params: &[String],
    body: &Statment,
    call_graph_edges: &mut Vec<(String, String)>,
) -> SymbolContextDigest {
    let mut is_stub = false;
    let mut stub_directive = None;
    let mut calls = Vec::new();
    let mut dataflow = Vec::new();
    let mut return_sources = Vec::new();
    let mut io_effects = Vec::new();
    let mut param_modes = HashMap::new();

    for p in params {
        param_modes.insert(p.clone(), ParamBorrowMode::ReadShared);
    }

    let mut inspector = InspectionContext {
        fn_name: name,
        params,
        param_modes: &mut param_modes,
        calls: &mut calls,
        dataflow: &mut dataflow,
        return_sources: &mut return_sources,
        io_effects: &mut io_effects,
        call_graph_edges,
    };

    // Inspect body for stubs and dataflow
    match body {
        Statment::BlockStatement(stmts) => {
            if stmts.is_empty() {
                is_stub = true;
                stub_directive = Some(format!("implement {}", name));
            } else if stmts.len() == 1 {
                if let Some(dir) = check_single_stmt_stub(&stmts[0]) {
                    is_stub = true;
                    stub_directive = Some(dir);
                }
            }
            for s in stmts {
                inspector.inspect_statement(s);
            }
        }
        _ => {
            if let Some(dir) = check_single_stmt_stub(body) {
                is_stub = true;
                stub_directive = Some(dir);
            }
            inspector.inspect_statement(body);
        }
    }

    if io_effects.is_empty() {
        io_effects.push("pure".to_string());
    } else {
        io_effects.sort();
        io_effects.dedup();
    }

    let param_list = params
        .iter()
        .map(|p| {
            (
                p.clone(),
                param_modes
                    .get(p)
                    .copied()
                    .unwrap_or(ParamBorrowMode::ReadShared),
            )
        })
        .collect();

    SymbolContextDigest {
        name: name.to_string(),
        kind: "fn".to_string(),
        params: param_list,
        is_stub,
        stub_directive,
        calls,
        io_effects,
        dataflow,
        return_sources,
    }
}

pub fn check_single_stmt_stub(stmt: &Statment) -> Option<String> {
    match stmt {
        Statment::ExpressionStatement(expr) => check_expr_stub(expr),
        Statment::PrintStatement(expr) => check_expr_stub(expr),
        _ => None,
    }
}

fn check_expr_stub(expr: &Expression) -> Option<String> {
    match expr {
        Expression::Call(name, args) => {
            let lower = name.to_lowercase();
            if lower == "stub"
                || lower == "todo"
                || lower == "unimplemented"
                || lower.starts_with("@stub")
            {
                if let Some(Expression::Literal(tok)) = args.first() {
                    return Some(tok.text.trim_matches('"').to_string());
                }
                return Some("unimplemented stub".to_string());
            }
            None
        }
        Expression::Literal(tok) if tok.text == "\"STUB\"" || tok.text == "STUB" => {
            Some("unimplemented stub".to_string())
        }
        _ => None,
    }
}

struct InspectionContext<'a> {
    fn_name: &'a str,
    params: &'a [String],
    param_modes: &'a mut HashMap<String, ParamBorrowMode>,
    calls: &'a mut Vec<String>,
    dataflow: &'a mut Vec<DataflowStep>,
    return_sources: &'a mut Vec<String>,
    io_effects: &'a mut Vec<String>,
    call_graph_edges: &'a mut Vec<(String, String)>,
}

impl<'a> InspectionContext<'a> {
    fn inspect_statement(&mut self, stmt: &Statment) {
        match stmt {
            Statment::ExpressionStatement(expr) => {
                self.inspect_expression(expr);
            }
            Statment::PrintStatement(expr) => {
                self.inspect_expression(expr);
            }
            Statment::LetStatement(tok, init) => {
                let mut src_vars = Vec::new();
                collect_vars_in_expr(init, &mut src_vars);
                self.inspect_expression(init);

                let op_name = summarize_expr(init);
                self.dataflow.push(DataflowStep {
                    source_vars: src_vars,
                    target: tok.text.clone(),
                    op_summary: op_name,
                });
            }
            Statment::IfStatement(cond, then_branch, else_branch) => {
                self.inspect_expression(cond);
                self.inspect_statement(then_branch);
                if let Some(eb) = else_branch {
                    self.inspect_statement(eb);
                }
            }
            Statment::WhileStatement(cond, body) => {
                self.inspect_expression(cond);
                self.inspect_statement(body);
            }
            Statment::ForStatement(_init, cond, inc, body) => {
                if let Some(c) = cond {
                    self.inspect_expression(c);
                }
                if let Some(i) = inc {
                    self.inspect_expression(i);
                }
                self.inspect_statement(body);
            }
            Statment::BlockStatement(stmts) => {
                for s in stmts {
                    self.inspect_statement(s);
                }
            }
            _ => {}
        }
    }

    fn inspect_expression(&mut self, expr: &Expression) {
        match expr {
            Expression::Assign(name, val) => {
                if self.params.contains(name) {
                    self.param_modes
                        .insert(name.clone(), ParamBorrowMode::MutateInPlace);
                }
                self.inspect_expression(val);
            }
            Expression::Set(target, _, val) => {
                if let Expression::Literal(tok) = &**target {
                    if self.params.contains(&tok.text) {
                        self.param_modes
                            .insert(tok.text.clone(), ParamBorrowMode::MutateInPlace);
                    }
                }
                self.inspect_expression(target);
                self.inspect_expression(val);
            }
            Expression::Call(callee, args) => {
                self.calls.push(callee.clone());
                self.call_graph_edges
                    .push((self.fn_name.to_string(), callee.clone()));

                match callee.as_str() {
                    "readFile" | "exists" | "isFile" | "isDir" | "listDir" | "load_json" => {
                        self.io_effects.push("fs:read".to_string());
                    }
                    "writeFile" | "mkdir" | "save_json" => {
                        self.io_effects.push("fs:write".to_string());
                    }
                    "persistent_kv" | "get" => {
                        self.io_effects.push("kv:read".to_string());
                    }
                    "set" | "del" => {
                        self.io_effects.push("kv:write".to_string());
                    }
                    "clock" => {
                        self.io_effects.push("clock".to_string());
                    }
                    "push" | "pop" => {
                        if let Some(Expression::Literal(tok)) = args.first() {
                            if self.params.contains(&tok.text) {
                                self.param_modes
                                    .insert(tok.text.clone(), ParamBorrowMode::MutateInPlace);
                            }
                        }
                    }
                    _ => {}
                }

                for arg in args {
                    self.inspect_expression(arg);
                }
            }
            Expression::Return(Some(ret_expr)) => {
                self.return_sources.push(summarize_expr(ret_expr));
                self.inspect_expression(ret_expr);
            }
            Expression::Binary(l, _, r) => {
                self.inspect_expression(l);
                self.inspect_expression(r);
            }
            Expression::Unary(_, r) => {
                self.inspect_expression(r);
            }
            Expression::Grouping(e) => {
                self.inspect_expression(e);
            }
            Expression::Array(items) => {
                for item in items {
                    self.inspect_expression(item);
                }
            }
            Expression::Index(l, r) => {
                self.inspect_expression(l);
                self.inspect_expression(r);
            }
            Expression::Object(pairs) => {
                for (_, val) in pairs {
                    self.inspect_expression(val);
                }
            }
            _ => {}
        }
    }
}

fn collect_vars_in_expr(expr: &Expression, vars: &mut Vec<String>) {
    match expr {
        Expression::Literal(tok) if tok.token_type == crate::tokens::TokenType::IDENTIFIER => {
            vars.push(tok.text.clone());
        }
        Expression::Binary(l, _, r) => {
            collect_vars_in_expr(l, vars);
            collect_vars_in_expr(r, vars);
        }
        Expression::Unary(_, r) => {
            collect_vars_in_expr(r, vars);
        }
        Expression::Grouping(e) => {
            collect_vars_in_expr(e, vars);
        }
        Expression::Call(_, args) => {
            for a in args {
                collect_vars_in_expr(a, vars);
            }
        }
        Expression::Array(items) => {
            for i in items {
                collect_vars_in_expr(i, vars);
            }
        }
        Expression::Index(l, r) => {
            collect_vars_in_expr(l, vars);
            collect_vars_in_expr(r, vars);
        }
        _ => {}
    }
}

fn summarize_expr(expr: &Expression) -> String {
    match expr {
        Expression::Binary(_, tok, _) => format!("binary `{}`", tok.text),
        Expression::Call(name, _) => format!("call `{}`", name),
        Expression::Literal(tok) => format!("literal `{}`", tok.text),
        Expression::Array(_) => "array literal".to_string(),
        Expression::Object(_) => "object literal".to_string(),
        _ => "expr".to_string(),
    }
}

pub fn generate_caveman_summary(sym: &SymbolContextDigest) -> String {
    if sym.is_stub {
        return format!(
            "STUB. Goal: {}.",
            sym.stub_directive
                .as_deref()
                .unwrap_or("no directive specified")
        );
    }

    let mut parts = Vec::new();

    // Side-effects / purity
    if sym.io_effects.contains(&"pure".to_string()) || sym.io_effects.is_empty() {
        parts.push("Pure function, zero side effects".to_string());
    } else {
        parts.push(format!("I/O effects: {}", sym.io_effects.join(", ")));
    }

    // Call dependencies
    if !sym.calls.is_empty() {
        parts.push(format!("Calls [{}]", sym.calls.join(", ")));
    }

    // Dataflow summary
    if !sym.dataflow.is_empty() {
        let flows = sym
            .dataflow
            .iter()
            .map(|df| format!("{} via {}", df.target, df.op_summary))
            .collect::<Vec<_>>()
            .join("; ");
        parts.push(format!("Computes: {}", flows));
    }

    // Return dependency
    if !sym.return_sources.is_empty() {
        parts.push(format!(
            "Returns derived from [{}]",
            sym.return_sources.join(", ")
        ));
    }

    if parts.is_empty() {
        "Verified symbol implementation".to_string()
    } else {
        parts.join(". ")
    }
}
pub fn format_natural_summary(sym: &SymbolContextDigest) -> String {
    let mut out = String::new();

    // Symbol signature
    let param_str = sym
        .params
        .iter()
        .map(|(p, _)| format!("{}:any", p))
        .collect::<Vec<_>>()
        .join(", ");
    out.push_str(&format!("symbol: {}({})", sym.name, param_str));
    if !sym.return_sources.is_empty() {
        out.push_str(" -> any");
    }
    out.push('\n');

    if sym.is_stub {
        out.push_str(&format!(
            "behavior: \"STUB: {}\"\n",
            sym.stub_directive.as_deref().unwrap_or("no directive")
        ));
        out.push_str("stub: true\n");
    } else {
        let mut behavior_parts = Vec::<String>::new();

        // Purity
        if sym.io_effects.contains(&"pure".to_string()) || sym.io_effects.is_empty() {
            behavior_parts.push("Pure function, zero side effects".to_string());
        } else {
            behavior_parts.push(format!("I/O effects: {}", sym.io_effects.join(", ")));
        }

        // Calls
        if !sym.calls.is_empty() {
            behavior_parts.push(format!("Calls [{}]", sym.calls.join(", ")));
        }

        // Dataflow
        if !sym.dataflow.is_empty() {
            let flows = sym
                .dataflow
                .iter()
                .map(|df| format!("{} via {}", df.target, df.op_summary))
                .collect::<Vec<_>>()
                .join("; ");
            behavior_parts.push(format!("Computes: {}", flows));
        }

        // Return sources
        if !sym.return_sources.is_empty() {
            behavior_parts.push(format!(
                "Returns derived from [{}]",
                sym.return_sources.join(", ")
            ));
        }

        let behavior = if behavior_parts.is_empty() {
            "Verified symbol implementation".to_string()
        } else {
            behavior_parts.join(". ")
        };
        out.push_str(&format!("behavior: \"{}\"\n", behavior));
        out.push_str("stub: false\n");
    }

    // Mutates (for future extension)
    out.push_str("mutates: []\n");

    // Calls list
    if !sym.calls.is_empty() {
        out.push_str(&format!("calls: [{}]\n", sym.calls.join(", ")));
    } else {
        out.push_str("calls: []\n");
    }

    out
}

pub fn format_compact_digest(digest: &ModuleContextDigest, filter_symbol: Option<&str>) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# PSUDO CONTEXT [{}] (~{}t | pIR: black-box)\n",
        digest.module_path, digest.estimated_tokens
    ));

    if !digest.diagnostics.is_empty() {
        out.push_str("diagnostics:\n");
        for d in &digest.diagnostics {
            out.push_str(&format!("  ! {}\n", d));
        }
    }

    if !digest.zettels.is_empty() {
        out.push_str("invariants (zettels):\n");
        for z in &digest.zettels {
            out.push_str(&format!("  * {}\n", z));
        }
    }

    out.push_str("symbols:\n");
    for s in &digest.symbols {
        if let Some(target) = filter_symbol {
            if s.name != target {
                continue;
            }
        }
        let param_str = s
            .params
            .iter()
            .map(|(p, m)| format!("{}{}:any", m.as_str(), p))
            .collect::<Vec<_>>()
            .join(", ");

        let status = if s.is_stub {
            format!(
                "STUB: \"{}\"",
                s.stub_directive.as_deref().unwrap_or("todo")
            )
        } else {
            "verified".to_string()
        };

        let io = s.io_effects.join(",");
        out.push_str(&format!(
            "  fn {}({}) [{} | {}]\n",
            s.name, param_str, status, io
        ));

        // When focusing on a specific symbol, emit caveman natural language behavioral summary
        if filter_symbol.is_some() {
            let summary = generate_caveman_summary(s);
            out.push_str(&format!("    summary: \"{}\"\n", summary));
        }

        if !s.dataflow.is_empty() {
            let flows = s
                .dataflow
                .iter()
                .map(|df| format!("({})->{}", df.source_vars.join("+"), df.target))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("    flow: {}\n", flows));
        }
        if !s.calls.is_empty() {
            out.push_str(&format!("    calls: [{}]\n", s.calls.join(", ")));
        }
    }

    if !digest.call_graph_edges.is_empty() {
        let edge_strs = digest
            .call_graph_edges
            .iter()
            .map(|(from, to)| format!("{}->{}", from, to))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("edges: {}\n", edge_strs));
    }

    out
}
