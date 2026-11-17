use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::blackbox::{synthesize_stubs_in_source, BlackBoxSynthRequest};
use crate::context::{analyze_psudo_source, ParamBorrowMode};
use crate::lexer::Lexer;
use crate::parser::{Expression, Parser, Statment};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustArtifactReport {
    pub binary_path: PathBuf,
    pub binary_size_bytes: u64,
    pub cargo_manifest: PathBuf,
    pub source_path: PathBuf,
    pub zero_unwrap_verified: bool,
    pub exit_code: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}

pub fn transpile_psudo_to_rust(source: &str, resolve_stubs: bool) -> Result<String, String> {
    let source_to_transpile = if resolve_stubs {
        let req = BlackBoxSynthRequest::default();
        let (synth_source, _) = synthesize_stubs_in_source(source, &req)?;
        synth_source
    } else {
        source.to_string()
    };

    let digest = analyze_psudo_source(&source_to_transpile, "memory.psudo", None)?;

    let mut lexer = Lexer::new(source_to_transpile);
    let tokens = lexer
        .scan_tokens()
        .map_err(|e| format!("Lexer error: {}", e))?;
    let mut parser = Parser::new(tokens);
    let statements = parser.parse().map_err(|e| format!("Parse error: {}", e))?;

    let mut out = String::new();
    out.push_str(get_rust_preamble());
    out.push('\n');

    let mut top_level_statements = Vec::new();
    let mut emitted_functions = std::collections::HashSet::new();

    for stmt in &statements {
        match stmt {
            Statment::ExpressionStatement(Expression::Function(params, body)) => {
                let fn_name = "anon".to_string();
                if emitted_functions.insert(fn_name.clone()) {
                    let fn_params = params.as_slice();
                    let sym_info = digest.symbols.iter().find(|s| s.name == fn_name);
                    let rendered_fn = render_rust_function(&fn_name, fn_params, body, sym_info);
                    out.push_str(&rendered_fn);
                    out.push('\n');
                }
            }
            Statment::ExpressionStatement(Expression::Assign(fn_name, expr))
                if matches!(**expr, Expression::Function(..)) =>
            {
                if emitted_functions.insert(fn_name.clone()) {
                    if let Expression::Function(params, body) = &**expr {
                        let fn_params = params.as_slice();
                        let sym_info = digest.symbols.iter().find(|s| &s.name == fn_name);
                        // I rename user `main` to `psudo_user_main` so the generated Rust harness can own
                        // the real `fn main()` entrypoint and wire up sandbox/panic-free runtime setup first.
                        let rust_fn_name = if fn_name == "main" {
                            "psudo_user_main"
                        } else {
                            fn_name.as_str()
                        };
                        let rendered_fn =
                            render_rust_function(rust_fn_name, fn_params, body, sym_info);
                        out.push_str(&rendered_fn);
                        out.push('\n');
                    }
                }
            }
            Statment::ClassStatement(name_tok, methods, _super_tok) => {
                let class_name = &name_tok.text;
                let rendered_class = render_rust_class(class_name, methods, &digest.symbols);
                out.push_str(&rendered_class);
                out.push('\n');
            }
            Statment::TestStatement(_, _) => {}
            _ => {
                top_level_statements.push(stmt);
            }
        }
    }

    // Render fn main()
    out.push_str("pub fn main() {\n");
    let mut calls_user_main = false;
    for s in top_level_statements {
        if let Statment::ExpressionStatement(Expression::Call(name, _)) = s {
            if name == "main" {
                calls_user_main = true;
            }
        }
        let line = render_rust_statement(s, 1);
        out.push_str(&line);
    }
    if emitted_functions.contains("main") && !calls_user_main {
        out.push_str("    let _ = psudo_user_main();\n");
    }
    out.push_str("}\n");

    Ok(out)
}

pub fn build_rust_binary_from_capsule(
    capsule: &crate::vfs::WorkspaceCapsule,
    out_dir: &Path,
    release: bool,
    run_after_build: bool,
) -> Result<RustArtifactReport, String> {
    let entry_src = capsule
        .virtual_files
        .get(&capsule.entrypoint)
        .cloned()
        .or_else(|| capsule.virtual_files.get("src/main.psudo").cloned())
        .ok_or_else(|| {
            "Workspace capsule does not contain an entrypoint source file".to_string()
        })?;

    let mut combined_source = String::new();
    combined_source.push_str(&entry_src);
    combined_source.push_str("\n\n");

    for (vpath, src) in &capsule.virtual_files {
        if vpath != &capsule.entrypoint && vpath != "src/main.psudo" && vpath.ends_with(".psudo") {
            combined_source.push_str(&format!("// --- Module: {} ---\n", vpath));
            combined_source.push_str(src);
            combined_source.push_str("\n\n");
        }
    }

    let rust_code = transpile_psudo_to_rust(&combined_source, true)?;

    let has_unwrap = rust_code.contains(".unwrap()") || rust_code.contains(".expect()");
    if has_unwrap {
        return Err("Transpiler emitted forbidden .unwrap() or .expect()".into());
    }

    std::fs::create_dir_all(out_dir)
        .map_err(|e| format!("Failed to create {}: {}", out_dir.display(), e))?;
    let src_dir = out_dir.join("src");
    std::fs::create_dir_all(&src_dir)
        .map_err(|e| format!("Failed to create {}: {}", src_dir.display(), e))?;

    let file_stem = capsule.project_name.replace('-', "_");

    let uses_serde = rust_code.contains("serde_json::");
    let deps_section = if uses_serde {
        "serde_json = \"1\"\nserde = { version = \"1\", features = [\"derive\"] }"
    } else {
        ""
    };

    let cargo_toml = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
{}

[[bin]]
name = "{}"
path = "src/main.rs"
"#,
        file_stem, deps_section, file_stem
    );

    let manifest_path = out_dir.join("Cargo.toml");
    std::fs::write(&manifest_path, cargo_toml)
        .map_err(|e| format!("Failed to write Cargo.toml: {}", e))?;

    let main_rs_path = src_dir.join("main.rs");
    std::fs::write(&main_rs_path, &rust_code)
        .map_err(|e| format!("Failed to write src/main.rs: {}", e))?;

    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("build")
        .arg("--offline")
        .arg("--manifest-path")
        .arg(&manifest_path);
    if release {
        cmd.arg("--release");
    }
    let mut output = cmd
        .output()
        .map_err(|e| format!("Cargo build failed to launch: {}", e))?;
    if !output.status.success() {
        let mut retry = std::process::Command::new("cargo");
        retry.arg("build").arg("--manifest-path").arg(&manifest_path);
        if release {
            retry.arg("--release");
        }
        output = retry
            .output()
            .map_err(|e| format!("Cargo build failed to launch: {}", e))?;
    }
    if !output.status.success() {
        return Err(format!(
            "Native binary build failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let bin_subdir = if release { "release" } else { "debug" };
    let binary_path = out_dir
        .join("target")
        .join(bin_subdir)
        .join(format!("{}.exe", file_stem));
    let binary_fallback = out_dir.join("target").join(bin_subdir).join(&file_stem);
    let compiled_bin = if binary_path.exists() {
        binary_path
    } else {
        binary_fallback
    };

    // If out_dir is inside a .pseudo folder (e.g. workspace/.pseudo/build), also mirror final executable to workspace/.pseudo/bin/
    let final_bin = if let Some(parent) = out_dir.parent() {
        if out_dir.file_name().map(|n| n == "build").unwrap_or(false)
            && parent.file_name().map(|n| n == ".pseudo").unwrap_or(false)
        {
            let bin_dest_dir = parent.join("bin");
            let _ = std::fs::create_dir_all(&bin_dest_dir);
            let bin_file_name = compiled_bin.file_name().unwrap_or_default();
            let dest_bin_path = bin_dest_dir.join(bin_file_name);
            if compiled_bin.exists() {
                let _ = std::fs::copy(&compiled_bin, &dest_bin_path);
            }
            if dest_bin_path.exists() {
                dest_bin_path
            } else {
                compiled_bin
            }
        } else {
            compiled_bin
        }
    } else {
        compiled_bin
    };

    let binary_size_bytes = std::fs::metadata(&final_bin).map(|m| m.len()).unwrap_or(0);

    let mut run_exit_code = None;
    let mut run_stdout = None;
    let mut run_stderr = None;

    if run_after_build && final_bin.exists() {
        let mut run_cmd = std::process::Command::new(&final_bin);
        if let Some(dot_pseudo) = out_dir.parent() {
            if out_dir.file_name().map(|n| n == "build").unwrap_or(false)
                && dot_pseudo.file_name().map(|n| n == ".pseudo").unwrap_or(false)
            {
                if let Some(ws_root) = dot_pseudo.parent() {
                    if !ws_root.as_os_str().is_empty() && ws_root.exists() {
                        run_cmd.current_dir(ws_root);
                    }
                }
            }
        }
        let run_output = run_cmd
            .output()
            .map_err(|e| format!("Failed to execute compiled binary: {}", e))?;
        run_exit_code = run_output.status.code();
        run_stdout = Some(String::from_utf8_lossy(&run_output.stdout).to_string());
        run_stderr = Some(String::from_utf8_lossy(&run_output.stderr).to_string());
    }

    Ok(RustArtifactReport {
        binary_path: final_bin,
        binary_size_bytes,
        cargo_manifest: manifest_path,
        source_path: PathBuf::from("psudo.bin"),
        zero_unwrap_verified: true,
        exit_code: run_exit_code,
        stdout: run_stdout,
        stderr: run_stderr,
    })
}
pub fn build_rust_binary_artifact(
    psudo_path: &Path,
    out_dir: &Path,
    release: bool,
    run_after_build: bool,
) -> Result<RustArtifactReport, String> {
    if psudo_path.exists()
        && psudo_path
            .extension()
            .map(|s| s == "psudo")
            .unwrap_or(false)
    {
        let source = std::fs::read_to_string(psudo_path)
            .map_err(|e| format!("Failed to read {}: {}", psudo_path.display(), e))?;
        let project_name = psudo_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("app");
        let mut capsule = crate::vfs::WorkspaceCapsule::new_empty(project_name);
        capsule
            .virtual_files
            .insert("src/main.psudo".to_string(), source);
        capsule.entrypoint = "src/main.psudo".to_string();
        let _ = capsule.recompute_context();
        build_rust_binary_from_capsule(&capsule, out_dir, release, run_after_build)
    } else {
        let (capsule_path, _) =
            crate::vfs::resolve_capsule_and_vpath(&psudo_path.to_string_lossy())?;
        let capsule = crate::vfs::WorkspaceCapsule::load_from_file(&capsule_path)?;
        build_rust_binary_from_capsule(&capsule, out_dir, release, run_after_build)
    }
}

fn render_rust_function(
    fn_name: &str,
    params: &[String],
    body: &Statment,
    sym_info: Option<&crate::context::SymbolContextDigest>,
) -> String {
    let mut out = String::new();
    let param_sigs = params
        .iter()
        .map(|p| {
            let mode = sym_info
                .and_then(|s| s.params.iter().find(|(name, _)| name == p).map(|(_, m)| *m))
                .unwrap_or(ParamBorrowMode::ReadShared);
            if mode == ParamBorrowMode::MutateInPlace {
                format!("mut {}: PsudoVal", p)
            } else {
                format!("{}: PsudoVal", p)
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    out.push_str(&format!(
        "pub fn {}({}) -> PsudoVal {{\n",
        fn_name, param_sigs
    ));
    let rendered_body = render_rust_statement(body, 1);
    out.push_str(&rendered_body);
    out.push_str("    PsudoVal::Null\n");
    out.push_str("}\n");
    out
}

fn render_rust_class(
    class_name: &str,
    methods: &[Statment],
    _symbols: &[crate::context::SymbolContextDigest],
) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "#[derive(Clone, Debug, Default)]\npub struct {} {{\n    pub fields: std::collections::BTreeMap<String, PsudoVal>,\n}}\n\n",
        class_name
    ));

    out.push_str(&format!("impl {} {{\n", class_name));
    out.push_str("    pub fn new() -> Self {\n        Self::default()\n    }\n");

    for m in methods {
        if let Statment::ExpressionStatement(Expression::Function(m_params, m_body)) = m {
            let m_name = m_params.first().cloned().unwrap_or_else(|| "m".to_string());
            let m_args = if m_params.len() > 1 {
                &m_params[1..]
            } else {
                &[]
            };

            let param_sigs = m_args
                .iter()
                .map(|p| format!("{}: PsudoVal", p))
                .collect::<Vec<_>>()
                .join(", ");

            let self_param = if param_sigs.is_empty() {
                "&mut self".to_string()
            } else {
                format!("&mut self, {}", param_sigs)
            };

            out.push_str(&format!(
                "    pub fn {}({}) -> PsudoVal {{\n",
                m_name, self_param
            ));
            let rendered_body = render_rust_statement(m_body, 2);
            out.push_str(&rendered_body);
            out.push_str("        PsudoVal::Null\n");
            out.push_str("    }\n");
        }
    }
    out.push_str("}\n");
    out
}

fn render_rust_statement(stmt: &Statment, indent: usize) -> String {
    let pad = "    ".repeat(indent);
    match stmt {
        Statment::PrintStatement(expr) => {
            format!("{}println!(\"{{}}\", {});\n", pad, render_rust_expr(expr))
        }
        Statment::LetStatement(tok, init) => {
            format!(
                "{}let mut {} = {};\n",
                pad,
                tok.text,
                render_rust_expr(init)
            )
        }
        Statment::ExpressionStatement(Expression::Assign(name, val)) => {
            format!("{}{} = {};\n", pad, name, render_rust_expr(val))
        }
        Statment::ExpressionStatement(Expression::Return(ret)) => {
            if let Some(r) = ret {
                format!("{}return {};\n", pad, render_rust_expr(r))
            } else {
                format!("{}return PsudoVal::Null;\n", pad)
            }
        }
        Statment::ExpressionStatement(expr) => {
            format!("{}{};\n", pad, render_rust_expr(expr))
        }
        Statment::IfStatement(cond, then_b, else_b) => {
            let mut out = format!("{}if psudo_truthy(&{}) {{\n", pad, render_rust_expr(cond));
            out.push_str(&render_rust_statement(then_b, indent + 1));
            if let Some(eb) = else_b {
                out.push_str(&format!("{}}} else {{\n", pad));
                out.push_str(&render_rust_statement(eb, indent + 1));
            }
            out.push_str(&format!("{}}}\n", pad));
            out
        }
        Statment::WhileStatement(cond, body) => {
            let mut out = format!(
                "{}while psudo_truthy(&{}) {{\n",
                pad,
                render_rust_expr(cond)
            );
            out.push_str(&render_rust_statement(body, indent + 1));
            out.push_str(&format!("{}}}\n", pad));
            out
        }
        Statment::ForStatement(init, cond, inc, body) => {
            let mut out = format!("{} {{\n", pad);
            let indent_pad = "    ".repeat(indent + 1);
            if let Some(init_stmt) = init {
                out.push_str(&render_rust_statement(init_stmt, indent + 1));
            }
            let cond_str = if let Some(c) = cond {
                render_rust_expr(c)
            } else {
                "PsudoVal::Bool(true)".to_string()
            };
            out.push_str(&format!(
                "{}while psudo_truthy(&{}) {{\n",
                indent_pad, cond_str
            ));
            out.push_str(&render_rust_statement(body, indent + 2));
            if let Some(inc_expr) = inc {
                out.push_str(&format!(
                    "{}{};\n",
                    "    ".repeat(indent + 2),
                    render_rust_expr(inc_expr)
                ));
            }
            out.push_str(&format!("{}}}\n", indent_pad));
            out.push_str(&format!("{}}}\n", pad));
            out
        }
        Statment::BlockStatement(stmts) => {
            let mut out = String::new();
            for s in stmts {
                out.push_str(&render_rust_statement(s, indent));
            }
            out
        }
        _ => String::new(),
    }
}

fn render_rust_expr(expr: &Expression) -> String {
    match expr {
        Expression::Literal(tok) => match tok.token_type {
            crate::tokens::TokenType::NUMBER => {
                let val: f32 = tok.text.parse().unwrap_or(0.0);
                format!("PsudoVal::Num({:.4})", val)
            }
            crate::tokens::TokenType::STRING => {
                format!("PsudoVal::Str({}.to_string())", tok.text)
            }
            crate::tokens::TokenType::TRUE => "PsudoVal::Bool(true)".to_string(),
            crate::tokens::TokenType::FALSE => "PsudoVal::Bool(false)".to_string(),
            crate::tokens::TokenType::NIL => "PsudoVal::Null".to_string(),
            _ => tok.text.clone(),
        },
        Expression::Binary(l, op, r) => match op.text.as_str() {
            "+" => format!(
                "psudo_add(&{}, &{})",
                render_rust_expr(l),
                render_rust_expr(r)
            ),
            "-" => format!(
                "psudo_sub(&{}, &{})",
                render_rust_expr(l),
                render_rust_expr(r)
            ),
            "*" => format!(
                "psudo_mul(&{}, &{})",
                render_rust_expr(l),
                render_rust_expr(r)
            ),
            "/" => format!(
                "psudo_div(&{}, &{})",
                render_rust_expr(l),
                render_rust_expr(r)
            ),
            "==" => format!(
                "PsudoVal::Bool({} == {})",
                render_rust_expr(l),
                render_rust_expr(r)
            ),
            "!=" => format!(
                "PsudoVal::Bool({} != {})",
                render_rust_expr(l),
                render_rust_expr(r)
            ),
            "<" => format!(
                "PsudoVal::Bool(psudo_cmp_less(&{}, &{}))",
                render_rust_expr(l),
                render_rust_expr(r)
            ),
            ">" => format!(
                "PsudoVal::Bool(psudo_cmp_greater(&{}, &{}))",
                render_rust_expr(l),
                render_rust_expr(r)
            ),
            "<=" => format!(
                "PsudoVal::Bool(!psudo_cmp_greater(&{}, &{}))",
                render_rust_expr(l),
                render_rust_expr(r)
            ),
            ">=" => format!(
                "PsudoVal::Bool(!psudo_cmp_less(&{}, &{}))",
                render_rust_expr(l),
                render_rust_expr(r)
            ),
            "and" => format!(
                "PsudoVal::Bool(psudo_truthy(&{}) && psudo_truthy(&{}))",
                render_rust_expr(l),
                render_rust_expr(r)
            ),
            "or" => format!(
                "PsudoVal::Bool(psudo_truthy(&{}) || psudo_truthy(&{}))",
                render_rust_expr(l),
                render_rust_expr(r)
            ),
            _ => format!(
                "psudo_add(&{}, &{})",
                render_rust_expr(l),
                render_rust_expr(r)
            ),
        },
        Expression::Unary(op, r) => match op.text.as_str() {
            "-" => format!("psudo_neg(&{})", render_rust_expr(r)),
            _ => render_rust_expr(r),
        },
        Expression::Grouping(e) => format!("({})", render_rust_expr(e)),
        Expression::Call(name, args) => {
            if name == "push" && args.len() == 2 {
                return format!(
                    "push(&mut {}, {})",
                    render_rust_expr(&args[0]),
                    render_rust_expr(&args[1])
                );
            }
            if name == "pop" && args.len() == 1 {
                return format!("pop(&mut {})", render_rust_expr(&args[0]));
            }
            if name == "assert" && args.len() == 1 {
                return format!("assert({}, PsudoVal::Null)", render_rust_expr(&args[0]));
            }
            if name == "assert_eq" && args.len() == 2 {
                return format!(
                    "assert_eq({}, {}, PsudoVal::Null)",
                    render_rust_expr(&args[0]),
                    render_rust_expr(&args[1])
                );
            }
            if name == "assert_ne" && args.len() == 2 {
                return format!(
                    "assert_ne({}, {}, PsudoVal::Null)",
                    render_rust_expr(&args[0]),
                    render_rust_expr(&args[1])
                );
            }
            if name == "slice" && args.len() == 2 {
                return format!(
                    "slice({}, {}, PsudoVal::Num(f32::MAX))",
                    render_rust_expr(&args[0]),
                    render_rust_expr(&args[1])
                );
            }
            let rendered_args = args
                .iter()
                .map(|a| {
                    if let Expression::Literal(tok) = a {
                        if tok.token_type == crate::tokens::TokenType::IDENTIFIER {
                            return format!("{}.clone()", tok.text);
                        }
                    }
                    render_rust_expr(a)
                })
                .collect::<Vec<_>>()
                .join(", ");
            let target_fn = if name == "main" {
                "psudo_user_main"
            } else {
                name.as_str()
            };
            format!("{}({})", target_fn, rendered_args)
        }
        Expression::Array(items) => {
            let item_strs = items
                .iter()
                .map(render_rust_expr)
                .collect::<Vec<_>>()
                .join(", ");
            format!("PsudoVal::List(vec![{}])", item_strs)
        }
        Expression::Object(pairs) => {
            let pair_strs = pairs
                .iter()
                .map(|(k, v)| format!("({:?}.to_string(), {})", k, render_rust_expr(v)))
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "PsudoVal::Map(std::collections::BTreeMap::from([{}]))",
                pair_strs
            )
        }
        Expression::Index(target, idx) => {
            format!(
                "psudo_index(&{}, &{})",
                render_rust_expr(target),
                render_rust_expr(idx)
            )
        }
        Expression::IndexSet(target, idx, val) => {
            if let Expression::Literal(tok) = target.as_ref() {
                format!(
                    "psudo_index_set(&mut {}, &{}, {})",
                    tok.text,
                    render_rust_expr(idx),
                    render_rust_expr(val)
                )
            } else {
                render_rust_expr(val)
            }
        }
        _ => "PsudoVal::Null".to_string(),
    }
}

fn get_rust_preamble() -> &'static str {
    r#"// Auto-generated by pseudo-transpiler (zero-unwrap production safe Rust)
#![allow(unused_variables, unused_mut, unreachable_code, dead_code)]

#[derive(Clone, Debug, PartialEq)]
pub enum PsudoVal {
    Num(f32),
    Bool(bool),
    Str(String),
    List(Vec<PsudoVal>),
    Map(std::collections::BTreeMap<String, PsudoVal>),
    Ok(Box<PsudoVal>),
    Err(Box<PsudoVal>),
    Null,
}

impl std::fmt::Display for PsudoVal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PsudoVal::Num(n) => {
                if n.fract() == 0.0 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }
            PsudoVal::Bool(b) => write!(f, "{}", b),
            PsudoVal::Str(s) => write!(f, "{}", s),
            PsudoVal::List(l) => {
                let items: Vec<String> = l.iter().map(|i| format!("{}", i)).collect();
                write!(f, "[{}]", items.join(", "))
            }
            PsudoVal::Map(m) => {
                let pairs: Vec<String> = m.iter().map(|(k, v)| format!("{}: {}", k, v)).collect();
                write!(f, "{{{}}}", pairs.join(", "))
            }
            PsudoVal::Ok(v) => write!(f, "Ok({})", v),
            PsudoVal::Err(e) => write!(f, "Err({})", e),
            PsudoVal::Null => write!(f, "null"),
        }
    }
}

pub fn psudo_truthy(v: &PsudoVal) -> bool {
    match v {
        PsudoVal::Bool(b) => *b,
        PsudoVal::Num(n) => *n != 0.0,
        PsudoVal::Str(s) => !s.is_empty(),
        PsudoVal::List(l) => !l.is_empty(),
        PsudoVal::Map(m) => !m.is_empty(),
        PsudoVal::Null => false,
        _ => true,
    }
}

pub fn psudo_add(a: &PsudoVal, b: &PsudoVal) -> PsudoVal {
    match (a, b) {
        (PsudoVal::Num(x), PsudoVal::Num(y)) => PsudoVal::Num(x + y),
        (PsudoVal::Str(x), PsudoVal::Str(y)) => PsudoVal::Str(format!("{}{}", x, y)),
        (PsudoVal::Str(x), PsudoVal::Num(y)) => PsudoVal::Str(format!("{}{}", x, y)),
        (PsudoVal::Num(x), PsudoVal::Str(y)) => PsudoVal::Str(format!("{}{}", x, y)),
        (PsudoVal::List(x), PsudoVal::List(y)) => {
            let mut combined = x.clone();
            combined.extend(y.clone());
            PsudoVal::List(combined)
        }
        _ => PsudoVal::Null,
    }
}

pub fn psudo_sub(a: &PsudoVal, b: &PsudoVal) -> PsudoVal {
    match (a, b) {
        (PsudoVal::Num(x), PsudoVal::Num(y)) => PsudoVal::Num(x - y),
        _ => PsudoVal::Null,
    }
}

pub fn psudo_mul(a: &PsudoVal, b: &PsudoVal) -> PsudoVal {
    match (a, b) {
        (PsudoVal::Num(x), PsudoVal::Num(y)) => PsudoVal::Num(x * y),
        _ => PsudoVal::Null,
    }
}

pub fn psudo_div(a: &PsudoVal, b: &PsudoVal) -> PsudoVal {
    match (a, b) {
        (PsudoVal::Num(x), PsudoVal::Num(y)) => {
            if *y == 0.0 {
                PsudoVal::Num(0.0)
            } else {
                PsudoVal::Num(x / y)
            }
        }
        _ => PsudoVal::Null,
    }
}

pub fn psudo_neg(v: &PsudoVal) -> PsudoVal {
    match v {
        PsudoVal::Num(n) => PsudoVal::Num(-n),
        _ => PsudoVal::Null,
    }
}

pub fn psudo_cmp_less(a: &PsudoVal, b: &PsudoVal) -> bool {
    match (a, b) {
        (PsudoVal::Num(x), PsudoVal::Num(y)) => x < y,
        (PsudoVal::Str(x), PsudoVal::Str(y)) => x < y,
        _ => false,
    }
}

pub fn psudo_cmp_greater(a: &PsudoVal, b: &PsudoVal) -> bool {
    match (a, b) {
        (PsudoVal::Num(x), PsudoVal::Num(y)) => x > y,
        (PsudoVal::Str(x), PsudoVal::Str(y)) => x > y,
        _ => false,
    }
}

pub fn len(v: PsudoVal) -> PsudoVal {
    match v {
        PsudoVal::List(l) => PsudoVal::Num(l.len() as f32),
        PsudoVal::Str(s) => PsudoVal::Num(s.len() as f32),
        PsudoVal::Map(m) => PsudoVal::Num(m.len() as f32),
        _ => PsudoVal::Num(0.0),
    }
}

pub fn str(v: PsudoVal) -> PsudoVal {
    PsudoVal::Str(format!("{}", v))
}

pub fn num(v: PsudoVal) -> PsudoVal {
    match v {
        PsudoVal::Num(n) => PsudoVal::Num(n),
        PsudoVal::Str(s) => PsudoVal::Num(s.parse().unwrap_or(0.0)),
        PsudoVal::Bool(b) => PsudoVal::Num(if b { 1.0 } else { 0.0 }),
        _ => PsudoVal::Num(0.0),
    }
}

pub fn bool(v: PsudoVal) -> PsudoVal {
    PsudoVal::Bool(psudo_truthy(&v))
}

pub fn clock() -> PsudoVal {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f32())
        .unwrap_or(0.0);
    PsudoVal::Num(now)
}

pub fn psudo_index(target: &PsudoVal, idx: &PsudoVal) -> PsudoVal {
    match target {
        PsudoVal::List(l) => {
            let i = match idx {
                PsudoVal::Num(n) => *n as usize,
                _ => 0,
            };
            if i < l.len() {
                l[i].clone()
            } else {
                PsudoVal::Null
            }
        }
        PsudoVal::Map(m) => {
            let k = match idx {
                PsudoVal::Str(s) => s.clone(),
                _ => "".to_string(),
            };
            m.get(&k).cloned().unwrap_or(PsudoVal::Null)
        }
        _ => PsudoVal::Null,
    }
}

pub fn psudo_index_set(target: &mut PsudoVal, idx: &PsudoVal, val: PsudoVal) -> PsudoVal {
    match target {
        PsudoVal::List(l) => {
            let i = match idx {
                PsudoVal::Num(n) => *n as usize,
                _ => 0,
            };
            if i < l.len() {
                l[i] = val.clone();
            }
            val
        }
        PsudoVal::Map(m) => {
            let k = format!("{}", idx);
            m.insert(k, val.clone());
            val
        }
        _ => val,
    }
}

pub fn push(arr: &mut PsudoVal, item: PsudoVal) -> PsudoVal {
    if let PsudoVal::List(l) = arr {
        l.push(item);
    }
    arr.clone()
}

pub fn pop(arr: &mut PsudoVal) -> PsudoVal {
    if let PsudoVal::List(l) = arr {
        return l.pop().unwrap_or(PsudoVal::Null);
    }
    PsudoVal::Null
}

pub fn slice(arr: PsudoVal, start: PsudoVal, end: PsudoVal) -> PsudoVal {
    if let PsudoVal::List(l) = arr {
        let s = match start { PsudoVal::Num(n) => n as usize, _ => 0 };
        let e = match end { PsudoVal::Num(n) => n as usize, _ => l.len() };
        let s = s.min(l.len());
        let e = e.min(l.len());
        if s <= e {
            return PsudoVal::List(l[s..e].to_vec());
        }
    }
    PsudoVal::List(vec![])
}

pub fn keys(obj: PsudoVal) -> PsudoVal {
    if let PsudoVal::Map(m) = obj {
        let ks: Vec<PsudoVal> = m.keys().map(|k| PsudoVal::Str(k.clone())).collect();
        return PsudoVal::List(ks);
    }
    PsudoVal::List(vec![])
}

pub fn values(obj: PsudoVal) -> PsudoVal {
    if let PsudoVal::Map(m) = obj {
        let vs: Vec<PsudoVal> = m.values().cloned().collect();
        return PsudoVal::List(vs);
    }
    PsudoVal::List(vec![])
}

pub fn has(obj: PsudoVal, key: PsudoVal) -> PsudoVal {
    if let (PsudoVal::Map(m), PsudoVal::Str(s)) = (obj, key) {
        return PsudoVal::Bool(m.contains_key(&s));
    }
    PsudoVal::Bool(false)
}

fn sanitize_sandbox_path(path_str: &str) -> std::path::PathBuf {
    let base = std::path::Path::new(".pseudo/sandbox");
    let cleaned = path_str.replace("..", "");
    let trimmed = cleaned.trim_start_matches('/').trim_start_matches('\\');
    base.join(trimmed)
}

pub fn readFile(path: PsudoVal) -> PsudoVal {
    let p_str = format!("{}", path);
    let target = sanitize_sandbox_path(&p_str);
    if let Ok(content) = std::fs::read_to_string(&target) {
        PsudoVal::Str(content)
    } else {
        PsudoVal::Str(String::new())
    }
}

pub fn writeFile(path: PsudoVal, content: PsudoVal) -> PsudoVal {
    let p_str = format!("{}", path);
    let c_str = format!("{}", content);
    let target = sanitize_sandbox_path(&p_str);
    if let Some(parent) = target.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&target, c_str);
    PsudoVal::Bool(true)
}

pub fn exists(path: PsudoVal) -> PsudoVal {
    let p_str = format!("{}", path);
    let target = sanitize_sandbox_path(&p_str);
    PsudoVal::Bool(target.exists())
}

pub fn isFile(path: PsudoVal) -> PsudoVal {
    let p_str = format!("{}", path);
    let target = sanitize_sandbox_path(&p_str);
    PsudoVal::Bool(target.is_file())
}

pub fn isDir(path: PsudoVal) -> PsudoVal {
    let p_str = format!("{}", path);
    let target = sanitize_sandbox_path(&p_str);
    PsudoVal::Bool(target.is_dir())
}

pub fn listDir(path: PsudoVal) -> PsudoVal {
    let p_str = format!("{}", path);
    let target = sanitize_sandbox_path(&p_str);
    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&target) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                names.push(PsudoVal::Str(name.to_string()));
            }
        }
    }
    PsudoVal::List(names)
}

pub fn mkdir(path: PsudoVal) -> PsudoVal {
    let p_str = format!("{}", path);
    let target = sanitize_sandbox_path(&p_str);
    let _ = std::fs::create_dir_all(&target);
    PsudoVal::Bool(true)
}
pub fn persistent_kv(store_name: PsudoVal) -> PsudoVal {
    let name = match store_name {
        PsudoVal::Str(s) => s,
        _ => "default_store".to_string(),
    };
    let mut map = std::collections::BTreeMap::new();
    map.insert("__type".to_string(), PsudoVal::Str("persistent_kv".to_string()));
    map.insert("store_name".to_string(), PsudoVal::Str(name));
    PsudoVal::Map(map)
}
pub fn get(store: PsudoVal, key: PsudoVal) -> PsudoVal {
    let store_name = match &store {
        PsudoVal::Map(m) => match m.get("store_name") {
            Some(PsudoVal::Str(s)) => s.clone(),
            _ => return PsudoVal::Num(0.0),
        },
        _ => return PsudoVal::Num(0.0),
    };
    let k = format!("{}", key);
    let path = std::path::PathBuf::from(format!(".pseudo/state/{}.json", store_name));
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(map) = serde_json::from_str::<std::collections::BTreeMap<String, serde_json::Value>>(&content) {
            if let Some(val) = map.get(&k) {
                return json_to_psudoval(val);
            }
        }
    }
    PsudoVal::Num(0.0)
}
pub fn set(store: PsudoVal, key: PsudoVal, val: PsudoVal) -> PsudoVal {
    let store_name = match &store {
        PsudoVal::Map(m) => match m.get("store_name") {
            Some(PsudoVal::Str(s)) => s.clone(),
            _ => return PsudoVal::Bool(false),
        },
        _ => return PsudoVal::Bool(false),
    };
    let k = format!("{}", key);
    let path = std::path::PathBuf::from(format!(".pseudo/state/{}.json", store_name));
    let mut map: std::collections::BTreeMap<String, serde_json::Value> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let jv = match val {
        PsudoVal::Num(n) => serde_json::json!(n),
        PsudoVal::Str(s) => serde_json::json!(s),
        PsudoVal::Bool(b) => serde_json::json!(b),
        _ => serde_json::json!(0),
    };
    map.insert(k, jv);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, serde_json::to_string(&map).unwrap_or_default());
    PsudoVal::Bool(true)
}
pub fn assert(cond: PsudoVal, msg: PsudoVal) -> PsudoVal {
    let b = match cond { PsudoVal::Bool(v) => v, PsudoVal::Num(n) => n != 0.0, _ => false };
    let m = match &msg { PsudoVal::Str(s) => s.clone(), _ => "assertion failed".to_string() };
    if b {
        PsudoVal::Ok(Box::new(PsudoVal::Bool(true)))
    } else {
        PsudoVal::Err(Box::new(PsudoVal::Str(m)))
    }
}
pub fn assert_eq(left: PsudoVal, right: PsudoVal, msg: PsudoVal) -> PsudoVal {
    let m = match &msg { PsudoVal::Str(s) => s.clone(), _ => format!("assertion failed: {} != {}", left, right) };
    if left == right {
        PsudoVal::Ok(Box::new(PsudoVal::Bool(true)))
    } else {
        PsudoVal::Err(Box::new(PsudoVal::Str(m)))
    }
}
pub fn assert_ne(left: PsudoVal, right: PsudoVal, msg: PsudoVal) -> PsudoVal {
    let m = match &msg { PsudoVal::Str(s) => s.clone(), _ => format!("assertion failed: {} == {}", left, right) };
    if left != right {
        PsudoVal::Ok(Box::new(PsudoVal::Bool(true)))
    } else {
        PsudoVal::Err(Box::new(PsudoVal::Str(m)))
    }
}
pub fn sqrt(x: PsudoVal) -> PsudoVal {
    let n: f32 = match x { PsudoVal::Num(v) => v, _ => 0.0 };
    PsudoVal::Num(n.sqrt())
}
pub fn pow(base: PsudoVal, exp: PsudoVal) -> PsudoVal {
    let b: f32 = match base { PsudoVal::Num(v) => v, _ => 0.0 };
    let e: f32 = match exp { PsudoVal::Num(v) => v, _ => 0.0 };
    PsudoVal::Num(b.powf(e))
}
pub fn abs(x: PsudoVal) -> PsudoVal {
    let n: f32 = match x { PsudoVal::Num(v) => v, _ => 0.0 };
    PsudoVal::Num(n.abs())
}
pub fn min(a: PsudoVal, b: PsudoVal) -> PsudoVal {
    let va: f32 = match a { PsudoVal::Num(v) => v, _ => 0.0 };
    let vb: f32 = match b { PsudoVal::Num(v) => v, _ => 0.0 };
    PsudoVal::Num(va.min(vb))
}
pub fn max(a: PsudoVal, b: PsudoVal) -> PsudoVal {
    let va: f32 = match a { PsudoVal::Num(v) => v, _ => 0.0 };
    let vb: f32 = match b { PsudoVal::Num(v) => v, _ => 0.0 };
    PsudoVal::Num(va.max(vb))
}
pub fn round(x: PsudoVal) -> PsudoVal {
    let n: f32 = match x { PsudoVal::Num(v) => v, _ => 0.0 };
    PsudoVal::Num(n.round())
}
pub fn floor(x: PsudoVal) -> PsudoVal {
    let n: f32 = match x { PsudoVal::Num(v) => v, _ => 0.0 };
    PsudoVal::Num(n.floor())
}
pub fn ceil(x: PsudoVal) -> PsudoVal {
    let n: f32 = match x { PsudoVal::Num(v) => v, _ => 0.0 };
    PsudoVal::Num(n.ceil())
}
pub fn time_now_sec() -> PsudoVal {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f32())
        .unwrap_or(0.0);
    PsudoVal::Num(now)
}
pub fn time_now_secs() -> PsudoVal {
    time_now_sec()
}
static START_INSTANT: std::sync::LazyLock<std::time::Instant> = std::sync::LazyLock::new(std::time::Instant::now);

pub fn time_micros() -> PsudoVal {
    PsudoVal::Num(START_INSTANT.elapsed().as_micros() as f32)
}
pub fn time_nanos() -> PsudoVal {
    PsudoVal::Num((START_INSTANT.elapsed().as_nanos() % 1_000_000_000) as f32)
}
pub fn sleep_ms(ms: PsudoVal) -> PsudoVal {
    let m = match ms { PsudoVal::Num(n) => n as u64, _ => 0 };
    std::thread::sleep(std::time::Duration::from_millis(m));
    PsudoVal::Bool(true)
}
pub fn benchmark<F: FnMut() -> PsudoVal>(iterations: PsudoVal, mut op: F) -> PsudoVal {
    let iters = match iterations { PsudoVal::Num(n) => n as u32, _ => 1 };
    let start = std::time::Instant::now();
    for _ in 0..iters {
        let _ = op();
    }
    let elapsed = start.elapsed();
    let total_us = elapsed.as_micros() as f32;
    let avg_us = if iters > 0 { total_us / (iters as f32) } else { 0.0 };

    let mut map = std::collections::BTreeMap::new();
    map.insert("iterations".to_string(), PsudoVal::Num(iters as f32));
    map.insert("total_us".to_string(), PsudoVal::Num(total_us));
    map.insert("avg_us".to_string(), PsudoVal::Num(avg_us));
    PsudoVal::Map(map)
}
pub fn deque_new(cap: PsudoVal) -> PsudoVal {
    PsudoVal::List(Vec::new())
}
pub fn push_front(arr: &mut PsudoVal, val: PsudoVal) -> PsudoVal {
    if let PsudoVal::List(l) = arr {
        l.insert(0, val);
    }
    PsudoVal::Null
}
pub fn pop_front(arr: &mut PsudoVal) -> PsudoVal {
    if let PsudoVal::List(l) = arr {
        if !l.is_empty() {
            return l.remove(0);
        }
    }
    PsudoVal::Num(0.0)
}
pub fn fmt_currency(amt: PsudoVal, sym: PsudoVal, decimals: PsudoVal) -> PsudoVal {
    let a: f32 = match amt { PsudoVal::Num(v) => v, _ => 0.0 };
    let s = match &sym { PsudoVal::Str(st) => st.clone(), _ => "$".to_string() };
    let d = match decimals { PsudoVal::Num(v) => v as usize, _ => 2 };
    PsudoVal::Str(format!("{}{:.*}", s, d, a))
}
pub fn load_json(path_val: PsudoVal) -> PsudoVal {
    let p_str = format!("{}", path_val);
    let path = std::path::Path::new(".pseudo/sandbox").join(&p_str);
    let content = std::fs::read_to_string(&path)
        .or_else(|_| std::fs::read_to_string(&p_str))
        .unwrap_or_else(|_| "{}".to_string());
    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&content) {
        json_to_psudoval(&json_val)
    } else {
        PsudoVal::Map(std::collections::BTreeMap::new())
    }
}
fn json_to_psudoval(val: &serde_json::Value) -> PsudoVal {
    match val {
        serde_json::Value::Null => PsudoVal::Num(0.0),
        serde_json::Value::Bool(b) => PsudoVal::Bool(*b),
        serde_json::Value::Number(n) => PsudoVal::Num(n.as_f64().unwrap_or(0.0) as f32),
        serde_json::Value::String(s) => PsudoVal::Str(s.clone()),
        serde_json::Value::Array(arr) => {
            PsudoVal::List(arr.iter().map(json_to_psudoval).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut map = std::collections::BTreeMap::new();
            for (k, v) in obj {
                map.insert(k.clone(), json_to_psudoval(v));
            }
            PsudoVal::Map(map)
        }
    }
}
 "#
}
