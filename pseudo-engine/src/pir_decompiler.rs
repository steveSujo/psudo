use crate::sampler::SparseChannel;
use psudo::parser::{Expression, Statment};
use psudo::pir_parser::parse_pir;

fn strip_ctx_header(input: &str) -> (&str, &str) {
    let trimmed = input.trim();
    if trimmed.starts_with("<ctx>") {
        if let Some(end) = trimmed.find("</ctx>") {
            let split_idx = end + "</ctx>".len();
            let header = &trimmed[..split_idx];
            let remainder = trimmed[split_idx..].trim_start();
            return (header, remainder);
        }
    }
    ("", trimmed)
}

// Routing prompts through compact pIR shrinks the token budget—fewer tokens generated
// means lower latency and less room for a small model to wander off-script.
pub fn route_prompt(user_input: &str, force_pir: bool) -> (SparseChannel, String) {
    let trimmed = user_input.trim();
    if force_pir {
        let routed = if trimmed.starts_with("Prompt:")
            || trimmed.starts_with("<pir>")
            || trimmed.starts_with("f ")
            || trimmed.starts_with("fn ")
            || trimmed.starts_with("<error>")
            || trimmed.contains("\n<pir>")
        {
            trimmed.to_string()
        } else {
            format!("Prompt: {}\n<pir>\n", trimmed)
        };
        return (SparseChannel::Pir, routed);
    }

    let (ctx_header, remainder) = strip_ctx_header(user_input);
    let ctx_prefix = if ctx_header.is_empty() {
        String::new()
    } else {
        format!("{}\n", ctx_header)
    };

    if remainder.starts_with("<pir>") {
        return (SparseChannel::Pir, format!("{}{}", ctx_prefix, remainder));
    }
    if remainder.starts_with("<ctx") {
        let routed = if remainder.ends_with("<pir>") {
            format!("{}{}", ctx_prefix, remainder)
        } else {
            format!("{}{}\n<pir>\n", ctx_prefix, remainder)
        };
        return (SparseChannel::Pir, routed);
    }
    if remainder.starts_with("f ") || remainder.starts_with("fn ") {
        let routed = if remainder.ends_with("<pir>") {
            format!("{}{}", ctx_prefix, remainder)
        } else {
            format!("{}{}\n<pir>\n", ctx_prefix, remainder)
        };
        return (SparseChannel::Pir, routed);
    }
    if let Some(msg) = remainder.strip_prefix("/code") {
        return (
            SparseChannel::Pir,
            format!("{}Prompt: {}\n<pir>\n", ctx_prefix, msg.trim()),
        );
    }
    if let Some(msg) = remainder.strip_prefix("/pir") {
        return (
            SparseChannel::Pir,
            format!("{}Prompt: {}\n<pir>\n", ctx_prefix, msg.trim()),
        );
    }
    if let Some(msg) = remainder.strip_prefix("/ask") {
        return (
            SparseChannel::Clarify,
            format!("{}Prompt: {}\n<clarify>\n", ctx_prefix, msg.trim()),
        );
    }
    if let Some(msg) = remainder.strip_prefix("/heal") {
        let body = msg.trim();
        let routed = if body.ends_with("<pir>") {
            format!("{}{}", ctx_prefix, body)
        } else {
            format!("{}{}\n<pir>\n", ctx_prefix, body)
        };
        return (SparseChannel::Pir, routed);
    }
    if remainder.starts_with("<error>") {
        let routed = if remainder.ends_with("<pir>") {
            format!("{}{}", ctx_prefix, remainder)
        } else {
            format!("{}{}\n<pir>\n", ctx_prefix, remainder)
        };
        return (SparseChannel::Pir, routed);
    }
    if let Some(rest) = remainder.strip_prefix("/zettel") {
        let arg = rest.trim();
        let routed = if arg.is_empty() {
            format!("{}<zettel>\n", ctx_prefix)
        } else {
            format!("{}/zettel {}\n<zettel>\n", ctx_prefix, arg)
        };
        return (SparseChannel::Zettel, routed);
    }
    if remainder.starts_with("<zettel>") {
        return (SparseChannel::Zettel, format!("{}{}", ctx_prefix, remainder));
    }

    let lower = remainder.to_lowercase();
    let code_keywords = [
        "fun ",
        "function",
        "write ",
        "implement",
        "create ",
        "build ",
        "add ",
        "calculate",
        "sum",
        "multiply",
        "filter",
        "redact",
        "parse",
        "validate",
        "sort",
        "loop",
        "ledger",
        "rate_limiter",
        "cache",
        "client",
        "return",
        "clamp",
        "cap",
        "fee",
        "compute",
        "check",
        "load",
        "count",
        "process",
        "if ",
        "alert",
        "incident",
        "sensor",
        "quota",
        "fib",
        "fibonacci",
        "is_",
        "apply_",
    ];
    if code_keywords.iter().any(|kw| lower.contains(kw)) {
        (
            SparseChannel::Pir,
            format!("{}Prompt: {}\n<pir>\n", ctx_prefix, remainder),
        )
    } else {
        (
            SparseChannel::Clarify,
            format!("{}Prompt: {}\n<clarify>\n", ctx_prefix, remainder),
        )
    }
}

pub fn format_expr(expr: &Expression) -> String {
    match expr {
        Expression::Literal(tok) => tok.text.clone(),
        Expression::Binary(left, op, right) => {
            let l_str = match left.as_ref() {
                Expression::Binary(..) => format!("({})", format_expr(left)),
                _ => format_expr(left),
            };
            let r_str = match right.as_ref() {
                Expression::Binary(..) => format!("({})", format_expr(right)),
                _ => format_expr(right),
            };
            format!("{} {} {}", l_str, op.text, r_str)
        }
        Expression::Unary(op, right) => {
            let r_str = match right.as_ref() {
                Expression::Binary(..) => format!("({})", format_expr(right)),
                _ => format_expr(right),
            };
            format!("{}{}", op.text, r_str)
        }
        Expression::Assign(name, val) => format!("{} = {}", name, format_expr(val)),
        Expression::Call(name, args) => {
            let arg_strs: Vec<String> = args.iter().map(format_expr).collect();
            format!("{}({})", name, arg_strs.join(", "))
        }
        Expression::Return(Some(val)) => format!("return {}", format_expr(val)),
        Expression::Return(None) => "return".to_string(),
        Expression::Array(elements) => {
            let elems: Vec<String> = elements.iter().map(format_expr).collect();
            format!("[{}]", elems.join(", "))
        }
        _ => "_expr_".to_string(),
    }
}

pub fn format_stmt(stmt: &Statment, indent: usize) -> String {
    let pad = "    ".repeat(indent);
    match stmt {
        Statment::BlockStatement(stmts) => {
            let mut out = String::new();
            for s in stmts {
                out.push_str(&format_stmt(s, indent));
            }
            out
        }
        Statment::LetStatement(tok, expr) => {
            format!("{}let {} = {}\n", pad, tok.text, format_expr(expr))
        }
        Statment::PrintStatement(expr) => {
            format!("{}print {}\n", pad, format_expr(expr))
        }
        Statment::IfStatement(cond, then_b, else_b) => {
            let mut out = format!(
                "{}if {}\n{}",
                pad,
                format_expr(cond),
                format_stmt(then_b, indent + 1)
            );
            if let Some(eb) = else_b {
                out.push_str(&format!("{}else\n{}", pad, format_stmt(eb, indent + 1)));
            }
            out
        }
        Statment::WhileStatement(cond, body) => {
            format!(
                "{}while {}\n{}",
                pad,
                format_expr(cond),
                format_stmt(body, indent + 1)
            )
        }
        Statment::ClassStatement(tok, methods, _) => {
            let mut out = format!("{}class {}\n", pad, tok.text);
            for m in methods {
                out.push_str(&format_stmt(m, indent + 1));
            }
            out
        }
        Statment::ExpressionStatement(expr) => match expr {
            Expression::Assign(name, val) => {
                if let Expression::Function(params, body) = val.as_ref() {
                    format!(
                        "{}fn {}({})\n{}",
                        pad,
                        name,
                        params.join(", "),
                        format_stmt(body, indent + 1)
                    )
                } else {
                    format!("{}{} = {}\n", pad, name, format_expr(val))
                }
            }
            Expression::Return(Some(val)) => {
                format!("{}return {}\n", pad, format_expr(val))
            }
            Expression::Return(None) => {
                format!("{}return\n", pad)
            }
            Expression::Call(name, args) => {
                let arg_strs: Vec<String> = args.iter().map(format_expr).collect();
                format!("{}{}({})\n", pad, name, arg_strs.join(", "))
            }
            _ => format!("{}{}\n", pad, format_expr(expr)),
        },
        _ => String::new(),
    }
}

pub fn decompile_pir_to_psudo(pir_text: &str) -> Option<String> {
    let mut all_tombstone = true;
    for line in pir_text.lines() {
        let s = line.trim();
        if s.is_empty() || s.starts_with("<pir>") || s.starts_with("</pir>") {
            continue;
        }
        if !s.starts_with("- ") {
            all_tombstone = false;
            break;
        }
    }
    if all_tombstone {
        let tomb_line = pir_text.lines().find(|l| l.trim().starts_with("- ")).unwrap_or("");
        return Some(format!("// CRDT tombstone: {}\n", tomb_line.trim()));
    }
    let clean_lines: Vec<&str> = pir_text
        .lines()
        .map(|l| l.trim())
        .filter(|l| {
            !l.is_empty()
                && !l.starts_with('!')
                && !l.starts_with("+ ")
                && !l.starts_with("- ")
                && !l.starts_with("<pir>")
                && !l.starts_with("</pir>")
                && !l.starts_with("//")
        })
        .collect();
    let clean_pir = clean_lines.join("\n");
    let stmts = parse_pir(&clean_pir).ok()?;
    let mut out = String::new();
    for stmt in &stmts {
        out.push_str(&format_stmt(stmt, 0));
    }
    if out.is_empty() {
        Some(pir_text.to_string())
    } else {
        let candidate = if out.starts_with("fn ") || out.starts_with("class ") {
            out
        } else {
            format!(
                "fn synthesized_func()\n{}",
                format_indented_body(&out)
            )
        };
        if !is_valid_decompiled_psudo(&candidate) {
            return None;
        }
        Some(candidate)
    }
}

fn is_valid_decompiled_psudo(source: &str) -> bool {
    let mut lexer = psudo::lexer::Lexer::new(source.to_string());
    let Ok(tokens) = lexer.scan_tokens() else {
        return false;
    };
    let mut parser = psudo::parser::Parser::new(tokens);
    let Ok(stmts) = parser.parse() else {
        return false;
    };
    // Reject hallucinated multi-arg FS builtin calls (e.g. mkdir with >2 args)
    fn has_invalid_arity(stmt: &Statment) -> bool {
        match stmt {
            Statment::ExpressionStatement(expr) | Statment::PrintStatement(expr) => expr_invalid_arity(expr),
            Statment::LetStatement(_, expr) => expr_invalid_arity(expr),
            Statment::BlockStatement(stmts) => stmts.iter().any(has_invalid_arity),
            Statment::IfStatement(c, t, e) => {
                expr_invalid_arity(c) || has_invalid_arity(t) || e.as_ref().is_some_and(|eb| has_invalid_arity(eb))
            }
            Statment::WhileStatement(c, b) => expr_invalid_arity(c) || has_invalid_arity(b),
            _ => false,
        }
    }
    fn expr_invalid_arity(expr: &Expression) -> bool {
        match expr {
            Expression::Call(name, args) => {
                let bad = matches!(name.as_str(), "mkdir" | "readFile" | "exists" | "isFile" | "isDir" | "listDir") && args.len() > 1
                    || name == "writeFile" && args.len() > 2;
                bad || args.iter().any(expr_invalid_arity)
            }
            Expression::Function(_, body) => has_invalid_arity(body),
            Expression::Assign(_, val) => expr_invalid_arity(val),
            Expression::Binary(l, _, r) => expr_invalid_arity(l) || expr_invalid_arity(r),
            Expression::Unary(_, r) | Expression::Grouping(r) => expr_invalid_arity(r),
            Expression::Return(Some(r)) => expr_invalid_arity(r),
            _ => false,
        }
    }
    !stmts.iter().any(has_invalid_arity)
}

fn format_indented_body(raw: &str) -> String {
    let mut indented = String::new();
    for line in raw.lines() {
        if line.is_empty() {
            indented.push('\n');
        } else {
            indented.push_str("    ");
            indented.push_str(line);
            indented.push('\n');
        }
    }
    indented
}
