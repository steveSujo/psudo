use fnv::FnvHasher;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use crate::vfs::WorkspaceCapsule;

/// Output contract matching the YAML schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    pub self_ref: String,
    pub media_type: String,
    pub prov: Provenance,
    pub signature: String,
    pub static_impl_summary: StaticImplSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    pub file_path: String,
    pub class_context: String,
    pub base_class: String,
    pub symbol: String,
    pub line_span: [usize; 2],
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticImplSummary {
    pub status: String,
    pub stub_directive: String,
    pub intent: String,
    pub guards: String,
    pub side_effects: String,
    pub sandbox_io: String,
    pub dependencies: String,
    pub control_flow: String,
}

/// Internal footprint extracted from function body
#[derive(Debug, Default)]
struct PseudoFootprint {
    is_stub: bool,
    stub_directives: Vec<String>,
    local_vars: HashSet<String>,
    guards: Vec<GuardClause>,
    mutations: HashSet<String>,
    sandbox_fs_ops: HashSet<String>,
    collection_ops: HashSet<String>,
    type_time_ops: HashSet<String>,
    user_calls: Vec<String>,
    max_loop_depth: u8,
    return_exprs: Vec<String>,
}

#[derive(Debug)]
struct GuardClause {
    cond: String,
    ret_expr: String,
}

#[derive(Debug, Clone)]
struct SanitizedLine {
    line_no: usize,
    indent: usize,
    text: String,
}

/// Sanitized source with string vault
pub struct SanitizedSource {
    lines: Vec<SanitizedLine>,
    str_vault: Vec<String>,
}

/// Extracted symbol info
pub struct ExtractedSymbol {
    name: String,
    params: Vec<String>,
    class_context: String,
    base_class: String,
    line_span: [usize; 2],
    body_lines: Vec<SanitizedLine>,
    pending_stub_directive: Option<String>,
    is_method: bool,
}

impl Contract {
    /// Generate contracts from a .pseudo source string
    pub fn generate_contracts(source: &str, file_path: &str) -> Result<Vec<Contract>, String> {
        let sanitized = sanitize_lines(source)?;
        let symbols = extract_symbols(&sanitized)?;
        let mut contracts = Vec::new();

        for sym in symbols {
            let footprint = analyze_pseudo_body(&sym, &sanitized.str_vault);
            let contract = build_contract(&sym, &footprint, file_path, &sanitized.str_vault);
            contracts.push(contract);
        }

        Ok(contracts)
    }

    /// Generate contracts from a WorkspaceCapsule
    pub fn generate_contracts_from_capsule(
        capsule: &WorkspaceCapsule,
    ) -> Result<Vec<Contract>, String> {
        let mut all_contracts = Vec::new();

        for (vpath, source) in &capsule.virtual_files {
            if vpath.ends_with(".psudo") || vpath.ends_with(".pseudo") {
                let mut contracts = Self::generate_contracts(source, vpath)?;
                for c in &mut contracts {
                    if c.static_impl_summary.status == "IMPLEMENTED" {
                        if let Some(traces) = capsule.stub_traces.get(&c.prov.symbol) {
                            if let Some(last_trace) =
                                traces.iter().rev().find(|t| !t.directive.is_empty())
                            {
                                c.static_impl_summary.stub_directive = last_trace.directive.clone();
                            }
                        }
                    }
                }
                all_contracts.extend(contracts);
            }
        }

        Ok(all_contracts)
    }

    /// Format contracts as YAML
    pub fn format_contracts_yaml(contracts: &[Contract]) -> String {
        serde_yaml::to_string(contracts).unwrap_or_default()
    }

    /// Format contracts as JSON
    pub fn format_contracts_json(contracts: &[Contract]) -> String {
        serde_json::to_string_pretty(contracts).unwrap_or_default()
    }
}

/// Stage 1: Line Normalizer & String Literal Vault
pub fn sanitize_lines(source: &str) -> Result<SanitizedSource, String> {
    let mut str_vault = Vec::new();
    let mut lines = Vec::new();
    let mut line_no = 0;

    for raw_line in source.lines() {
        line_no += 1;

        // Strip comments
        let line = if let Some(idx) = raw_line.find('#') {
            &raw_line[..idx]
        } else if let Some(idx) = raw_line.find("//") {
            &raw_line[..idx]
        } else {
            raw_line
        };

        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }

        // Count leading spaces (indent)
        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim_start();

        // Extract string literals
        let sanitized_text = extract_strings(trimmed, &mut str_vault);

        lines.push(SanitizedLine {
            line_no,
            indent,
            text: sanitized_text,
        });
    }

    Ok(SanitizedSource { lines, str_vault })
}

/// Extract string literals, replace with __STR_k__ placeholders
fn extract_strings(line: &str, str_vault: &mut Vec<String>) -> String {
    let mut result = String::new();
    let chars = line.chars().peekable();
    let mut in_string = false;
    let mut escape = false;
    let mut current_string = String::new();

    for ch in chars {
        if !in_string {
            if ch == '"' {
                in_string = true;
                current_string.clear();
                // Don't store opening quote
            } else {
                result.push(ch);
            }
        } else {
            if escape {
                current_string.push(ch);
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                // End of string - don't store closing quote
                in_string = false;
                let idx = str_vault.len();
                str_vault.push(current_string.clone());
                result.push_str(&format!("__STR_{}__", idx));
            } else {
                current_string.push(ch);
            }
        }
    }

    result
}

/// Stage 2: Indentation Scope Slicer
pub fn extract_symbols(sanitized: &SanitizedSource) -> Result<Vec<ExtractedSymbol>, String> {
    let mut symbols = Vec::new();
    let mut class_stack: Vec<(usize, String, String)> = Vec::new(); // (indent, class_name, base_class)
    let mut pending_stub_directive: Option<String> = None;
    let mut i = 0;

    while i < sanitized.lines.len() {
        let line = &sanitized.lines[i];
        let text = &line.text;

        // Pop class stack when dedenting
        while let Some((class_indent, _, _)) = class_stack.last() {
            if line.indent <= *class_indent && !text.is_empty() {
                class_stack.pop();
            } else {
                break;
            }
        }

        // Check for class declaration
        if let Some((class_name, base_class)) = parse_class_declaration(text) {
            class_stack.push((line.indent, class_name.clone(), base_class.clone()));
            i += 1;
            continue;
        }

        // Check for @stub or todo decorator
        if let Some(directive) = parse_stub_decorator(text, &sanitized.str_vault) {
            pending_stub_directive = Some(directive);
            i += 1;
            continue;
        }

        // Check for function declaration
        if let Some((fn_name, params)) = parse_function_declaration(text) {
            let (class_context, base_class) = if let Some((_, cn, bc)) = class_stack.last() {
                (cn.clone(), bc.clone())
            } else {
                (String::new(), String::new())
            };

            let start_line = line.line_no;
            let fn_indent = line.indent;
            let is_method = !class_context.is_empty();

            // Collect body lines (indent > fn_indent)
            let mut body_lines = Vec::new();
            let mut j = i + 1;
            while j < sanitized.lines.len() {
                let bl = &sanitized.lines[j];
                if bl.indent <= fn_indent && !bl.text.trim().is_empty() {
                    break;
                }
                if !bl.text.trim().is_empty() {
                    body_lines.push(bl.clone());
                }
                j += 1;
            }

            let end_line = if body_lines.is_empty() {
                start_line
            } else {
                body_lines.last().unwrap().line_no
            };

            symbols.push(ExtractedSymbol {
                name: fn_name,
                params,
                class_context,
                base_class,
                line_span: [start_line, end_line],
                body_lines,
                pending_stub_directive: pending_stub_directive.take(),
                is_method,
            });
        }

        i += 1;
    }

    Ok(symbols)
}

fn parse_class_declaration(text: &str) -> Option<(String, String)> {
    // class Name < Base
    let text = text.trim();
    if !text.starts_with("class ") {
        return None;
    }
    let rest = &text[6..].trim();
    if let Some(idx) = rest.find('<') {
        let name = rest[..idx].trim().to_string();
        let base = rest[idx + 1..]
            .trim()
            .trim_end_matches('>')
            .trim()
            .to_string();
        Some((name, base))
    } else {
        Some((rest.to_string(), String::new()))
    }
}

fn parse_stub_decorator(text: &str, str_vault: &[String]) -> Option<String> {
    let text = text.trim();
    // @stub("__STR_k__") or todo("__STR_k__")
    if text.starts_with("@stub(") && text.ends_with(")") {
        let inner = &text[6..text.len() - 1].trim();
        if inner.starts_with("__STR_") && inner.ends_with("__") {
            let idx_str = &inner[6..inner.len() - 2];
            if let Ok(idx) = idx_str.parse::<usize>() {
                return str_vault.get(idx).cloned();
            }
        }
        return Some(inner.trim_matches('"').to_string());
    }
    if text.starts_with("todo(") && text.ends_with(")") {
        let inner = &text[5..text.len() - 1].trim();
        if inner.starts_with("__STR_") && inner.ends_with("__") {
            let idx_str = &inner[6..inner.len() - 2];
            if let Ok(idx) = idx_str.parse::<usize>() {
                return str_vault.get(idx).cloned();
            }
        }
        return Some(inner.trim_matches('"').to_string());
    }
    None
}

fn parse_function_declaration(text: &str) -> Option<(String, Vec<String>)> {
    let text = text.trim();
    if !text.starts_with("fn ") {
        return None;
    }
    let rest = &text[3..].trim();
    let paren_end = rest.find(')')?;
    let name_and_params = &rest[..=paren_end];

    if let Some(paren_start) = name_and_params.find('(') {
        let name = name_and_params[..paren_start].trim().to_string();
        let params_str = &name_and_params[paren_start + 1..name_and_params.len() - 1].trim();
        let params: Vec<String> = if params_str.is_empty() {
            Vec::new()
        } else {
            params_str
                .split(',')
                .map(|s| s.trim().to_string())
                .collect()
        };
        Some((name, params))
    } else {
        None
    }
}

/// Stage 3: Footprint Extractor
fn analyze_pseudo_body(sym: &ExtractedSymbol, str_vault: &[String]) -> PseudoFootprint {
    let mut footprint = PseudoFootprint::default();
    let mut loop_indent_stack: Vec<usize> = Vec::new();
    let mut first_stmt = true;
    let mut pending_guard_cond: Option<String> = None;
    let mut for_loop_skip = 0; // Counter to skip for loop header lines (condition, step)

    if let Some(directive) = &sym.pending_stub_directive {
        footprint.is_stub = true;
        footprint.stub_directives.push(directive.clone());
    }

    for line in &sym.body_lines {
        let text = &line.text;
        let indent = line.indent;

        // Pop loop stack on dedent
        while let Some(&loop_indent) = loop_indent_stack.last() {
            if indent <= loop_indent {
                loop_indent_stack.pop();
            } else {
                break;
            }
        }

        // Handle for loop header skip
        if for_loop_skip > 0 {
            // Classify calls in skipped lines
            classify_calls(text, &mut footprint);
            for_loop_skip -= 1;
            first_stmt = false;
            continue;
        }

        // Check for stub/todo inside body
        if let Some(directive) = parse_stub_decorator(text, str_vault) {
            footprint.is_stub = true;
            footprint.stub_directives.push(directive);
        }

        // Check for let binding
        if let Some(var) = parse_let_binding(text) {
            footprint.local_vars.insert(var);
            // Also classify calls in the RHS of let bindings
            classify_calls(text, &mut footprint);
            first_stmt = false;
            continue;
        }

        // Check for while loop
        if text.trim_start().starts_with("while ") {
            loop_indent_stack.push(indent);
            footprint.max_loop_depth = footprint.max_loop_depth.max(loop_indent_stack.len() as u8);
            first_stmt = false;
            continue;
        }

        // Check for for loop (3-line header)
        if text.trim_start().starts_with("for let ") {
            loop_indent_stack.push(indent);
            footprint.max_loop_depth = footprint.max_loop_depth.max(loop_indent_stack.len() as u8);
            // Classify calls in the for init line
            classify_calls(text, &mut footprint);
            // Skip next 2 lines at child indent (condition and step)
            for_loop_skip = 2;
            first_stmt = false;
            continue;
        }

        // Check for if statement (guard clause): supports both `if (cond)` and `if cond`
        let trimmed_if = text.trim_start();
        if first_stmt && (trimmed_if.starts_with("if (") || trimmed_if.starts_with("if ")) {
            let cond = if trimmed_if.starts_with("if (") {
                let after_if = &trimmed_if[4..];
                let mut paren_count = 1;
                let mut cond_end = None;
                for (i, ch) in after_if.char_indices() {
                    if ch == '(' {
                        paren_count += 1;
                    } else if ch == ')' {
                        paren_count -= 1;
                        if paren_count == 0 {
                            cond_end = Some(i);
                            break;
                        }
                    }
                }
                cond_end.map(|rel_end| after_if[..rel_end].trim().to_string())
            } else {
                Some(trimmed_if[3..].trim().trim_end_matches(':').trim().to_string())
            };
            if let Some(cond) = cond {
                classify_calls(text, &mut footprint);
                pending_guard_cond = Some(cond);
                first_stmt = false;
                continue;
            }
        }

        // Check if this line completes a pending guard clause (return or todo)
        if let Some(cond) = pending_guard_cond.take() {
            if text.contains("return ") {
                let ret_start = text.find("return ").unwrap() + 7;
                let ret = text[ret_start..].trim().to_string();
                footprint.guards.push(GuardClause {
                    cond,
                    ret_expr: ret,
                });
                first_stmt = false;
                continue;
            } else if text.contains("todo(") {
                footprint.guards.push(GuardClause {
                    cond,
                    ret_expr: "todo(...)".to_string(),
                });
                first_stmt = false;
                continue;
            } else {
                // Not a guard clause, clear pending
                pending_guard_cond = None;
            }
        }

        // Check for return
        if let Some(ret) = parse_return(text) {
            footprint.return_exprs.push(ret);
        }

        // Check for assignments/mutations
        if let Some(lhs) = parse_assignment(text) {
            if !footprint.local_vars.contains(&lhs)
                || lhs.starts_with("this.")
                || lhs.starts_with("self.")
                || lhs.contains('[')
            {
                footprint.mutations.insert(lhs);
            }
        }

        // Check for push/pop mutations
        if let Some(arr) = parse_push_pop(text) {
            footprint
                .mutations
                .insert(format!("{} (via push/pop)", arr));
        }

        // Classify calls
        classify_calls(text, &mut footprint);

        first_stmt = false;
    }

    footprint
}

fn parse_let_binding(text: &str) -> Option<String> {
    let text = text.trim();
    if text.starts_with("let ") {
        let rest = &text[4..].trim();
        if let Some(eq_idx) = rest.find('=') {
            return Some(rest[..eq_idx].trim().to_string());
        }
    }
    None
}

pub fn parse_guard_clause(text: &str) -> Option<(String, String)> {
    let text = text.trim();
    if text.starts_with("if (") {
        let cond_end = text.find(')')?;
        let cond = text[4..cond_end].trim().to_string();
        // Look for return or todo in the same line or next (simplified)
        if text.contains("return ") {
            let ret_start = text.find("return ")? + 7;
            let ret = text[ret_start..].trim().to_string();
            return Some((cond, ret));
        }
        if text.contains("todo(") {
            return Some((cond, "todo(...)".to_string()));
        }
    }
    None
}

fn parse_return(text: &str) -> Option<String> {
    let text = text.trim();
    if text.starts_with("return ") {
        Some(text[7..].trim().to_string())
    } else if text == "return" {
        Some("nil".to_string())
    } else {
        None
    }
}

fn parse_assignment(text: &str) -> Option<String> {
    let text = text.trim();
    // Skip let, ==, !=, <=, >=
    if text.starts_with("let ")
        || text.contains("==")
        || text.contains("!=")
        || text.contains("<=")
        || text.contains(">=")
    {
        return None;
    }
    if let Some(eq_idx) = text.find('=') {
        // Check it's not ==
        if eq_idx + 1 < text.len() && text.as_bytes()[eq_idx + 1] == b'=' {
            return None;
        }
        let lhs = text[..eq_idx].trim();
        if !lhs.is_empty() {
            return Some(lhs.to_string());
        }
    }
    None
}

fn parse_push_pop(text: &str) -> Option<String> {
    let text = text.trim();
    if text.starts_with("push(") {
        let inner = &text[5..text.len() - 1];
        let comma = inner.find(',')?;
        Some(inner[..comma].trim().to_string())
    } else if text.starts_with("pop(") {
        let inner = &text[4..text.len() - 1];
        Some(inner.trim().to_string())
    } else {
        None
    }
}

/// Builtin taxonomy
const SANDBOX_FS: &[&str] = &[
    "readFile",
    "writeFile",
    "listDir",
    "exists",
    "isFile",
    "isDir",
    "mkdir",
    "load_json",
    "save_json",
];
const SANDBOX_KV: &[&str] = &["persistent_kv", "get", "set", "del"];
const COLLECTION_MUT: &[&str] = &["push", "pop"];
const COLLECTION_READ: &[&str] = &["len", "slice", "keys", "values", "has"];
const RUNTIME_UTIL: &[&str] = &["str", "num", "bool", "clock", "print"];

fn classify_calls(text: &str, footprint: &mut PseudoFootprint) {
    let text = text.trim();

    for &builtin in SANDBOX_FS {
        if text.contains(&format!("{builtin}(")) {
            footprint.sandbox_fs_ops.insert(builtin.to_string());
        }
    }
    for &builtin in SANDBOX_KV {
        if text.contains(&format!("{builtin}(")) {
            footprint.sandbox_fs_ops.insert(builtin.to_string());
            if builtin == "set" || builtin == "del" {
                footprint
                    .mutations
                    .insert(format!("persistent_kv (via {})", builtin));
            }
        }
    }
    for &builtin in COLLECTION_MUT {
        if text.contains(&format!("{builtin}(")) {
            footprint.collection_ops.insert(builtin.to_string());
        }
    }
    for &builtin in COLLECTION_READ {
        if text.contains(&format!("{builtin}(")) {
            footprint.collection_ops.insert(builtin.to_string());
        }
    }
    for &builtin in RUNTIME_UTIL {
        if text.contains(&format!("{builtin}("))
            || (builtin == "print" && text.starts_with("print "))
        {
            footprint.type_time_ops.insert(builtin.to_string());
        }
    }
    if text.contains("super.") {
        // Extract super.method()
        if let Some(idx) = text.find("super.") {
            let rest = &text[idx + 6..];
            if let Some(paren) = rest.find('(') {
                let method = rest[..paren].trim().to_string();
                let call = format!("super.{}", method);
                if !footprint.user_calls.contains(&call) {
                    footprint.user_calls.push(call);
                }
            }
        }
    }

    // Detect user-defined function calls `ident(` excluding keywords and builtins
    let bytes = text.as_bytes();
    for (idx, &b) in bytes.iter().enumerate() {
        if b == b'(' && idx > 0 {
            let mut start = idx;
            while start > 0
                && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_')
            {
                start -= 1;
            }
            if start < idx {
                let is_method_on_obj = start > 0 && bytes[start - 1] == b'.';
                let ident = &text[start..idx];
                let is_builtin = SANDBOX_FS.contains(&ident)
                    || SANDBOX_KV.contains(&ident)
                    || COLLECTION_MUT.contains(&ident)
                    || COLLECTION_READ.contains(&ident)
                    || RUNTIME_UTIL.contains(&ident)
                    || matches!(ident, "if" | "while" | "for" | "fn" | "return" | "todo");
                if !is_method_on_obj
                    && !is_builtin
                    && !ident.is_empty()
                    && !footprint.user_calls.contains(&ident.to_string())
                {
                    footprint.user_calls.push(ident.to_string());
                }
            }
        }
    }
}

/// Stage 4: Verbalizer & Contract Builder
fn build_contract(
    sym: &ExtractedSymbol,
    footprint: &PseudoFootprint,
    file_path: &str,
    _str_vault: &[String],
) -> Contract {
    // Compute content hash (FNV-1a 64-bit)
    let mut hasher = FnvHasher::default();
    for line in &sym.body_lines {
        line.text.hash(&mut hasher);
    }
    let content_hash = format!("0x{:016x}", hasher.finish());

    // Status
    let status = if footprint.is_stub
        && footprint.user_calls.is_empty()
        && footprint.sandbox_fs_ops.is_empty()
    {
        "BLACK_BOX_STUB"
    } else if footprint.is_stub {
        "PARTIAL_IMPL_WITH_TODO"
    } else {
        "IMPLEMENTED"
    }
    .to_string();

    // Stub directive
    let stub_directive = if footprint.is_stub {
        footprint.stub_directives.join("; ")
    } else {
        "None".to_string()
    };

    // Intent
    let verb = verb_from_fn_name(&sym.name);
    let rest = rest_from_fn_name(&sym.name);
    let params_str = sym.params.join(", ");
    let intent = if sym.is_method {
        format!(
            "Method on {} (inherits {}): {} {} with params ({}).",
            sym.class_context,
            if sym.base_class.is_empty() {
                "Object"
            } else {
                &sym.base_class
            },
            verb,
            rest,
            params_str
        )
    } else {
        format!("{} {} with params ({}).", verb, rest, params_str)
    };

    // Guards
    let guards = if footprint.guards.is_empty() {
        "None (unconditional entry).".to_string()
    } else {
        let g = &footprint.guards[0];
        format!(
            "Returns {} immediately if {}.",
            g.ret_expr,
            verbalize_pseudo_expr(&g.cond)
        )
    };

    // Side effects
    let side_effects = if footprint.mutations.is_empty()
        && !footprint
            .collection_ops
            .iter()
            .any(|c| COLLECTION_MUT.contains(&c.as_str()))
    {
        "Pure (local state only).".to_string()
    } else {
        let mut parts = Vec::new();
        if !footprint.mutations.is_empty() {
            let mut muts: Vec<_> = footprint.mutations.iter().cloned().collect();
            muts.sort();
            parts.push(format!("Mutates {}", muts.join(", ")));
        }
        let mut mut_coll: Vec<_> = footprint
            .collection_ops
            .iter()
            .filter(|c| COLLECTION_MUT.contains(&c.as_str()))
            .cloned()
            .collect();
        mut_coll.sort();
        if !mut_coll.is_empty() {
            parts.push(format!(
                "modifies collections via [{}]",
                mut_coll.join(", ")
            ));
        }
        parts.join("; ") + "."
    };

    // Sandbox I/O
    let sandbox_io = if footprint.sandbox_fs_ops.is_empty() {
        "Zero filesystem operations.".to_string()
    } else {
        let mut ops: Vec<_> = footprint.sandbox_fs_ops.iter().cloned().collect();
        ops.sort();
        format!("Confined to .pseudo/sandbox/: invokes [{}].", ops.join(", "))
    };

    // Dependencies
    let mut deps = Vec::new();
    if !footprint.user_calls.is_empty() {
        deps.push(format!("Calls {}", footprint.user_calls.join(", ")));
    }
    let read_coll: Vec<_> = footprint
        .collection_ops
        .iter()
        .filter(|c| COLLECTION_READ.contains(&c.as_str()))
        .cloned()
        .collect();
    let mut util: Vec<_> = footprint.type_time_ops.iter().cloned().collect();
    util.sort();
    if !read_coll.is_empty() || !util.is_empty() {
        let mut builtins = read_coll;
        builtins.extend(util);
        deps.push(format!("uses builtins [{}]", builtins.join(", ")));
    }
    let dependencies = if deps.is_empty() {
        "Leaf function (zero internal calls).".to_string()
    } else {
        deps.join("; ")
    };

    // Control flow
    let control_flow = match footprint.max_loop_depth {
        0 => "Linear (0 loops)".to_string(),
        1 => "Single-loop (depth 1)".to_string(),
        d => format!("Nested loops (depth {})", d),
    };

    // Signature
    let signature = format!("fn {}({})", sym.name, sym.params.join(", "));

    Contract {
        self_ref: format!(
            "#/repos/core/{}#{}",
            file_path,
            if sym.is_method {
                format!("{}.{}", sym.class_context, sym.name)
            } else {
                sym.name.clone()
            }
        ),
        media_type: "pseudo_ast_stub".to_string(),
        prov: Provenance {
            file_path: file_path.to_string(),
            class_context: if sym.class_context.is_empty() {
                "nil".to_string()
            } else {
                sym.class_context.clone()
            },
            base_class: if sym.base_class.is_empty() {
                "nil".to_string()
            } else {
                sym.base_class.clone()
            },
            symbol: sym.name.clone(),
            line_span: sym.line_span,
            content_hash,
        },
        signature,
        static_impl_summary: StaticImplSummary {
            status,
            stub_directive,
            intent,
            guards,
            side_effects,
            sandbox_io,
            dependencies,
            control_flow,
        },
    }
}

fn verb_from_fn_name(name: &str) -> String {
    // Convert snake_case to verb phrase
    let parts: Vec<&str> = name.split('_').collect();
    if parts.is_empty() {
        return name.to_string();
    }
    parts[0].to_string()
}

fn rest_from_fn_name(name: &str) -> String {
    let parts: Vec<&str> = name.split('_').collect();
    if parts.len() <= 1 {
        return String::new();
    }
    parts[1..].join(" ")
}

/// Verbalize pseudo expressions deterministically
fn verbalize_pseudo_expr(expr: &str) -> String {
    let expr = expr.trim();

    // Handle negated builtin calls first (e.g., !isDir(...), !exists(...))
    if expr.starts_with('!') {
        let inner = expr[1..].trim();
        // Check for negated builtin calls
        if inner.starts_with("isDir(") && inner.ends_with(')') {
            let path = &inner[6..inner.len() - 1];
            return format!("not sandbox path {} is a directory", verbalize_atom(path));
        }
        if inner.starts_with("isFile(") && inner.ends_with(')') {
            let path = &inner[7..inner.len() - 1];
            return format!("not sandbox path {} is a file", verbalize_atom(path));
        }
        if inner.starts_with("exists(") && inner.ends_with(')') {
            let path = &inner[7..inner.len() - 1];
            return format!("sandbox path {} does not exist", verbalize_atom(path));
        }
        if inner.starts_with("has(") && inner.ends_with(')') {
            let inner2 = &inner[4..inner.len() - 1];
            let comma = inner2.find(',').unwrap_or(0);
            let obj = inner2[..comma].trim();
            let key = inner2[comma + 1..].trim();
            return format!(
                "object {} does not have key {}",
                verbalize_atom(obj),
                verbalize_atom(key)
            );
        }
        // Generic not
        return format!("not {}", verbalize_pseudo_expr(inner));
    }

    // Handle builtin calls (positive)
    if expr.starts_with("isDir(") && expr.ends_with(')') {
        let path = &expr[6..expr.len() - 1];
        return format!("sandbox path {} is a directory", verbalize_atom(path));
    }
    if expr.starts_with("isFile(") && expr.ends_with(')') {
        let path = &expr[7..expr.len() - 1];
        return format!("sandbox path {} is a file", verbalize_atom(path));
    }
    if expr.starts_with("exists(") && expr.ends_with(')') {
        let path = &expr[7..expr.len() - 1];
        return format!("sandbox path {} exists", verbalize_atom(path));
    }
    if expr.starts_with("has(") && expr.ends_with(')') {
        let inner = &expr[4..expr.len() - 1];
        let comma = inner.find(',').unwrap_or(0);
        let obj = inner[..comma].trim();
        let key = inner[comma + 1..].trim();
        return format!(
            "object {} has key {}",
            verbalize_atom(obj),
            verbalize_atom(key)
        );
    }
    if expr.contains("len(") && expr.ends_with(" == 0") {
        let inner = &expr[4..expr.len() - 6].trim_end_matches(')');
        return format!("{} length is zero", verbalize_atom(inner));
    }

    // Handle binary operators
    for (op, verbal) in &[
        ("==", "equals"),
        ("!=", "does not equal"),
        ("<=", "is at most"),
        (">=", "is at least"),
        ("<", "is less than"),
        (">", "exceeds"),
    ] {
        if let Some(idx) = expr.find(op) {
            let left = expr[..idx].trim();
            let right = expr[idx + op.len()..].trim();
            return format!(
                "{} {} {}",
                verbalize_atom(left),
                verbal,
                verbalize_atom(right)
            );
        }
    }

    expr.to_string()
}

fn verbalize_atom(atom: &str) -> String {
    let atom = atom.trim();
    if atom.starts_with('"') && atom.ends_with('"') {
        format!("\"{}\"", &atom[1..atom.len() - 1])
    } else if atom == "true" || atom == "false" || atom == "nil" || atom.parse::<f64>().is_ok() {
        atom.to_string()
    } else {
        atom.to_string()
    }
}
