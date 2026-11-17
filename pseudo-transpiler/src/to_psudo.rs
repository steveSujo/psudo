use crate::ir::TsNode;

// Re-lexing and re-parsing the emitted `.psudo` before returning catches indentation
// or syntax drift at the IR boundary instead of failing downstream.
pub fn ir_to_psudo(root: &TsNode) -> Result<String, String> {
    let mut out = String::new();
    for child in &root.children {
        let rendered = render_node(child, 0)?;
        if !rendered.is_empty() {
            out.push_str(&rendered);
            out.push('\n');
        }
    }
    let psudo_str = out.clone();
    let mut lexer = psudo::lexer::Lexer::new(psudo_str.clone());
    if let Ok(tokens) = lexer.scan_tokens() {
        let mut parser = psudo::parser::Parser::new(tokens);
        if let Err(e) = parser.parse() {
            return Err(format!("Emitted invalid .psudo syntax: {:?}\nSource:\n{}", e, psudo_str));
        }
    }
    Ok(out)
}

fn render_node(node: &TsNode, indent: usize) -> Result<String, String> {
    let pad = "    ".repeat(indent);
    match node.kind.as_str() {
        "source_file" => {
            let mut out = String::new();
            for child in &node.children {
                let r = render_node(child, indent)?;
                if !r.is_empty() {
                    out.push_str(&r);
                    out.push('\n');
                }
            }
            Ok(out)
        }
        "function_definition" | "function_item" => {
            let name = node.children.iter()
                .find(|c| c.field.as_deref() == Some("name") || c.kind == "identifier")
                .and_then(|c| c.value.clone())
                .unwrap_or_else(|| {
                    node.children.iter().find(|c| c.kind == "identifier").and_then(|c| c.value.clone()).unwrap_or_else(|| "anon".to_string())
                });

            let params_node = node.children.iter().find(|c| c.kind == "parameters" || c.kind == "parameter_list");
            let mut params = Vec::new();
            if let Some(pn) = params_node {
                for p in &pn.children {
                    if p.kind == "identifier" || p.kind == "parameter" {
                        if let Some(id) = p.children.iter().find(|c| c.kind == "identifier").and_then(|c| c.value.clone()) {
                            params.push(id);
                        } else if let Some(v) = &p.value {
                            let clean_name = v.split(':').next().unwrap_or(v).trim().to_string();
                            if !clean_name.is_empty() && clean_name != ":" {
                                params.push(clean_name);
                            }
                        }
                    }
                }
            }

            let body_node = node.children.iter()
                .find(|c| c.field.as_deref() == Some("body") || c.kind == "block" || c.kind == "compound_statement")
                .unwrap_or(node);

            let body_str = render_node(body_node, indent + 1)?;
            Ok(format!("{}fn {}({})\n{}", pad, name, params.join(", "), body_str))
        }
        "test_item" => {
            let test_name = node.value.clone().unwrap_or_else(|| "test".to_string());
            let body_node = node.children.iter()
                .find(|c| c.field.as_deref() == Some("body") || c.kind == "block")
                .ok_or_else(|| "Missing test body".to_string())?;
            let body_str = render_node(body_node, indent + 1)?;
            Ok(format!("{}test \"{}\"\n{}", pad, test_name, body_str))
        }
        "block" | "compound_statement" => {
            let mut out = String::new();
            for child in &node.children {
                if child.kind == "{" || child.kind == "}" || child.kind == ";" {
                    continue;
                }
                let r = render_node(child, indent)?;
                if !r.is_empty() {
                    out.push_str(&r);
                    out.push('\n');
                }
            }
            if out.is_empty() {
                out.push_str(&format!("{}    return\n", pad));
            }
            Ok(out)
        }
        "let_declaration" => {
            let pattern = node.children.iter()
                .find(|c| c.field.as_deref() == Some("pattern") || c.kind == "identifier")
                .and_then(|c| c.value.clone())
                .unwrap_or_else(|| "x".to_string());
            let clean_pat = pattern.split(':').next().unwrap_or(&pattern).trim().to_string();
            let mut clean_pat = clean_pat;
            if clean_pat.starts_with("mut ") {
                clean_pat = clean_pat.trim_start_matches("mut ").trim().to_string();
            }
            let value_node = node.children.iter()
                .find(|c| c.field.as_deref() == Some("value"))
                .ok_or_else(|| "Missing let value".to_string())?;
            Ok(format!("{}let {} = {}", pad, clean_pat, render_expr(value_node)?))
        }
        "expression_statement" => {
            if let Some(child) = node.children.first() {
                if child.kind == ";" {
                    return Ok("".to_string());
                }
                Ok(format!("{}{}", pad, render_expr(child)?))
            } else {
                Ok("".to_string())
            }
        }
        "macro_invocation" => {
            Ok(format!("{}{}", pad, render_expr(node)?))
        }
        _ => {
            if node.kind == "{" || node.kind == "}" || node.kind == ";" {
                return Ok("".to_string());
            }
            if let Some(v) = &node.value {
                Ok(format!("{}{}", pad, v))
            } else {
                let mut out = String::new();
                for child in &node.children {
                    if child.kind == "{" || child.kind == "}" || child.kind == ";" {
                        continue;
                    }
                    let r = render_node(child, indent)?;
                    if !r.is_empty() {
                        out.push_str(&r);
                        out.push('\n');
                    }
                }
                Ok(out)
            }
        }
    }
}

fn render_expr(node: &TsNode) -> Result<String, String> {
    match node.kind.as_str() {
        "binary_expression" => {
            let left = node.children.iter().find(|c| c.field.as_deref() == Some("left")).ok_or("Missing left")?;
            let right = node.children.iter().find(|c| c.field.as_deref() == Some("right")).ok_or("Missing right")?;
            
            let op = node.children.iter()
                .find(|c| c.field.is_none() && c.kind != "identifier" && c.kind != "integer_literal" && c.kind != "call_expression" && c.kind != "binary_expression" && c.kind != "left" && c.kind != "right")
                .and_then(|c| c.value.clone())
                .or_else(|| {
                    if let Some(v) = &node.value {
                        if v.contains('+') { Some("+".to_string()) }
                        else if v.contains('-') { Some("-".to_string()) }
                        else if v.contains('*') { Some("*".to_string()) }
                        else if v.contains('/') { Some("/".to_string()) }
                        else { None }
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| "+".to_string());

            let op_symbol = match op.as_str() {
                "add" | "add(&" => "+",
                "sub" | "sub(&" => "-",
                "mul" | "mul(&" => "*",
                "div" | "div(&" => "/",
                _ => &op,
            };
            Ok(format!("{} {} {}", render_expr(left)?, op_symbol, render_expr(right)?))
        }
        "call_expression" => {
            let func = node.children.iter().find(|c| c.field.as_deref() == Some("function")).ok_or("Missing function")?;
            let args_node = node.children.iter().find(|c| c.kind == "arguments" || c.kind == "argument_list").ok_or("Missing arguments")?;
            let mut args = Vec::new();
            for arg in &args_node.children {
                if arg.kind != "(" && arg.kind != ")" && arg.kind != "," {
                    args.push(render_expr(arg)?);
                }
            }
            let fname = func.value.clone().unwrap_or_else(|| "fn".to_string());
            Ok(format!("{}({})", fname, args.join(", ")))
        }
        "macro_invocation" => {
            let args_node = node.children.iter().find(|c| c.kind == "arguments" || c.kind == "token_tree");
            let mut raw_args = Vec::new();
            if let Some(an) = args_node {
                for arg in &an.children {
                    if arg.kind != "(" && arg.kind != ")" && arg.kind != "," {
                        raw_args.push(render_expr(arg)?);
                    }
                }
            }
            // Filter out format strings if multiple arguments exist and format string is "{}" or "{:?}"
            let mut args = Vec::new();
            let has_multiple = raw_args.len() > 1;
            for (idx, arg) in raw_args.into_iter().enumerate() {
                let trimmed = arg.trim_matches('"').trim_matches('\'');
                if has_multiple && idx == 0 && (trimmed == "{}" || trimmed == "{:?}") {
                    continue;
                }
                args.push(arg);
            }
            let expr_str = if !args.is_empty() { args.join(", ") } else { "".to_string() };
            Ok(format!("print {}", expr_str))
        }
        "integer_literal" | "string_literal" | "boolean_literal" | "identifier" | "number_literal" => {
            let mut val = node.value.clone().unwrap_or_default();
            if val.starts_with("PsudoVal::Num(") && val.ends_with(".0)") {
                val = val.trim_start_matches("PsudoVal::Num(").trim_end_matches(".0)").to_string();
            } else if val.starts_with("PsudoVal::Str(") && val.ends_with(".to_string())") {
                val = val.trim_start_matches("PsudoVal::Str(").trim_end_matches(".to_string()").to_string();
            } else if val.starts_with("PsudoVal::Bool(") && val.ends_with(")") {
                val = val.trim_start_matches("PsudoVal::Bool(").trim_end_matches(")").to_string();
            }
            Ok(val)
        }
        "return_expression" => {
            let mut exprs = Vec::new();
            for child in &node.children {
                if child.kind != "return" && child.kind != ";" {
                    let r = render_expr(child)?;
                    if !r.is_empty() {
                        exprs.push(r);
                    }
                }
            }
            if !exprs.is_empty() {
                Ok(format!("return {}", exprs.join(" ")))
            } else {
                Ok("return".to_string())
            }
        }
        _ => {
            let mut left_expr = None;
            let mut right_expr = None;
            let mut op_str = "+";
            for child in &node.children {
                if child.field.as_deref() == Some("left") {
                    left_expr = Some(render_expr(child)?);
                } else if child.field.as_deref() == Some("right") {
                    right_expr = Some(render_expr(child)?)
                } else if child.kind == "+" || child.kind == "-" || child.kind == "*" || child.kind == "/" {
                    op_str = &child.kind;
                }
            }
            if let (Some(l), Some(r)) = (left_expr, right_expr) {
                return Ok(format!("{} {} {}", l, op_str, r));
            }
            Ok(node.value.clone().unwrap_or_default())
        }
    }
}
