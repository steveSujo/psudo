use crate::ir::TsNode;

pub fn ir_to_rust(root: &TsNode) -> Result<String, String> {
    if root.kind != "source_file" {
        return Err(format!("Expected source_file root, got {}", root.kind));
    }

    let mut out = String::new();
    out.push_str(RUST_PREAMBLE);
    out.push('\n');

    let mut top_level_statements = Vec::new();
    let mut functions = Vec::new();

    for child in &root.children {
        match child.kind.as_str() {
            "function_item" => {
                functions.push(render_function(child)?);
            }
            "test_item" => {
                functions.push(render_test(child)?);
            }
            _ => {
                top_level_statements.push(child);
            }
        }
    }

    for f in functions {
        out.push_str(&f);
        out.push('\n');
    }

    out.push_str("pub fn main() {\n");
    for stmt in top_level_statements {
        out.push_str(&format!("    {};\n", render_node(stmt)?));
    }
    out.push_str("}\n");

    Ok(out)
}

fn render_function(node: &TsNode) -> Result<String, String> {
    let name = node.children.iter()
        .find(|c| c.field.as_deref() == Some("name") || c.kind == "identifier")
        .and_then(|c| c.value.clone())
        .unwrap_or_else(|| "anon".to_string());

    let params_node = node.children.iter()
        .find(|c| c.kind == "parameters");

    let mut params = Vec::new();
    if let Some(pn) = params_node {
        for p in &pn.children {
            if let Some(id) = p.children.iter().find(|c| c.kind == "identifier").and_then(|c| c.value.clone()) {
                params.push(format!("{}: PsudoVal", id));
            } else if let Some(v) = &p.value {
                params.push(format!("{}: PsudoVal", v));
            }
        }
    }

    let body_node = node.children.iter()
        .find(|c| c.field.as_deref() == Some("body") || c.kind == "block")
        .ok_or_else(|| "Missing body block in function_item".to_string())?;

    let body_str = render_block(body_node)?;

    Ok(format!("pub fn {}({}) -> PsudoVal {{\n{}\n}}\n", name, params.join(", "), body_str))
}

fn render_test(node: &TsNode) -> Result<String, String> {
    let test_name = node.value.clone().unwrap_or_else(|| "unnamed_test".to_string());
    let body_node = node.children.iter()
        .find(|c| c.field.as_deref() == Some("body") || c.kind == "block")
        .ok_or_else(|| "Missing body in test_item".to_string())?;
    let body_str = render_block(body_node)?;

    Ok(format!("#[test]\npub fn test_{}() {{\n{}\n}}\n", test_name.replace(' ', "_"), body_str))
}

fn render_block(node: &TsNode) -> Result<String, String> {
    let mut out = String::new();
    for child in &node.children {
        out.push_str(&format!("    {};\n", render_node(child)?));
    }
    if out.is_empty() {
        out.push_str("    PsudoVal::Nil\n");
    }
    Ok(out)
}

fn render_node(node: &TsNode) -> Result<String, String> {
    match node.kind.as_str() {
        "let_declaration" => {
            let pattern = node.children.iter()
                .find(|c| c.field.as_deref() == Some("pattern") || c.kind == "identifier")
                .and_then(|c| c.value.clone())
                .unwrap_or_else(|| "x".to_string());
            let value = node.children.iter()
                .find(|c| c.field.as_deref() == Some("value") || c.kind != "identifier")
                .ok_or_else(|| "Missing value in let_declaration".to_string())?;
            Ok(format!("let mut {} = {}", pattern, render_node(value)?))
        }
        "binary_expression" => {
            let left = node.children.iter().find(|c| c.field.as_deref() == Some("left")).ok_or("Missing binary left")?;
            let right = node.children.iter().find(|c| c.field.as_deref() == Some("right")).ok_or("Missing binary right")?;
            let op = node.value.as_deref().unwrap_or("+");
            let method = match op {
                "+" => "add",
                "-" => "sub",
                "*" => "mul",
                "/" => "div",
                _ => "add",
            };
            Ok(format!("{}.{}(&{})", render_node(left)?, method, render_node(right)?))
        }
        "call_expression" => {
            let func = node.children.iter().find(|c| c.field.as_deref() == Some("function")).ok_or("Missing call function")?;
            let args_node = node.children.iter().find(|c| c.kind == "arguments").ok_or("Missing call arguments")?;
            let mut args = Vec::new();
            for arg in &args_node.children {
                args.push(render_node(arg)?);
            }
            let fname = func.value.clone().unwrap_or_else(|| "unknown".to_string());
            Ok(format!("{}({})", fname, args.join(", ")))
        }
        "macro_invocation" => {
            let macro_name = node.value.as_deref().unwrap_or("println!");
            let args_node = node.children.iter().find(|c| c.kind == "arguments").ok_or("Missing macro args")?;
            let mut args = Vec::new();
            for arg in &args_node.children {
                args.push(render_node(arg)?);
            }
            if macro_name == "println!" {
                Ok(format!("println!(\"{{}}\", {})", args.join(", ")))
            } else {
                Ok(format!("{}!({})", macro_name.trim_end_matches('!'), args.join(", ")))
            }
        }
        "integer_literal" => {
            let val = node.value.as_deref().unwrap_or("0");
            Ok(format!("PsudoVal::Num({}.0)", val))
        }
        "string_literal" => {
            let val = node.value.as_deref().unwrap_or("");
            Ok(format!("PsudoVal::Str({}.to_string())", val))
        }
        "boolean_literal" => {
            let val = node.value.as_deref().unwrap_or("false");
            Ok(format!("PsudoVal::Bool({})", val))
        }
        "identifier" => {
            let val = node.value.as_deref().unwrap_or("x");
            Ok(val.to_string())
        }
        "return_expression" => {
            if let Some(child) = node.children.first() {
                Ok(format!("return {}", render_node(child)?))
            } else {
                Ok("return PsudoVal::Nil".to_string())
            }
        }
        _ => {
            // Fallback for any unmapped expression children
            if let Some(v) = &node.value {
                Ok(v.clone())
            } else {
                Ok("PsudoVal::Nil".to_string())
            }
        }
    }
}

const RUST_PREAMBLE: &str = r#"
#[derive(Debug, Clone, PartialEq)]
pub enum PsudoVal {
    Num(f64),
    Str(String),
    Bool(bool),
    Nil,
}

impl PsudoVal {
    pub fn add(&self, other: &PsudoVal) -> PsudoVal {
        match (self, other) {
            (PsudoVal::Num(a), PsudoVal::Num(b)) => PsudoVal::Num(a + b),
            (PsudoVal::Str(a), PsudoVal::Str(b)) => PsudoVal::Str(format!("{}{}", a, b)),
            _ => PsudoVal::Nil,
        }
    }
    pub fn sub(&self, other: &PsudoVal) -> PsudoVal {
        match (self, other) {
            (PsudoVal::Num(a), PsudoVal::Num(b)) => PsudoVal::Num(a - b),
            _ => PsudoVal::Nil,
        }
    }
    pub fn mul(&self, other: &PsudoVal) -> PsudoVal {
        match (self, other) {
            (PsudoVal::Num(a), PsudoVal::Num(b)) => PsudoVal::Num(a * b),
            _ => PsudoVal::Nil,
        }
    }
    pub fn div(&self, other: &PsudoVal) -> PsudoVal {
        match (self, other) {
            (PsudoVal::Num(a), PsudoVal::Num(b)) if *b != 0.0 => PsudoVal::Num(a / b),
            _ => PsudoVal::Nil,
        }
    }
}

impl std::fmt::Display for PsudoVal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PsudoVal::Num(n) => write!(f, "{}", n),
            PsudoVal::Str(s) => write!(f, "{}", s),
            PsudoVal::Bool(b) => write!(f, "{}", b),
            PsudoVal::Nil => write!(f, "nil"),
        }
    }
}
"#;
