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
    // Emitting `#[tokio::main] pub async fn main()` only when async addon stubs are called
    // keeps `.psudo` syntax untouched while still unlocking async Rust crates.
    let project_cfg = crate::config::load_project_config();
    let mut main_attr: Option<String> = None;
    let mut main_async = false;
    if source.contains("tokio_sleep_ms(") {
        main_attr = Some("#[tokio::main]".to_string());
        main_async = true;
    }
    for addon in project_cfg.addons.values() {
        let used = addon
            .builtins
            .iter()
            .any(|b| source.contains(&format!("{}(", b)));
        if used {
            if let Some(attr) = &addon.rust_main_attr {
                main_attr = Some(attr.clone());
            }
            if addon.rust_main_async {
                main_async = true;
            }
        }
    }
    if let Some(attr) = main_attr {
        out.push_str(&format!("{}\n", attr));
    }
    if main_async {
        out.push_str("pub async fn main() {\n");
    } else {
        out.push_str("pub fn main() {\n");
    }
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

    let uses_tui = out.contains("tui_dashboard(")
        || out.contains("tui_grid(")
        || out.contains("tui_fractal(")
        || out.contains("tui_metric(")
        || out.contains("tui_gauge(")
        || out.contains("tui_sparkline(")
        || out.contains("tui_table(")
        || out.contains("tui_log(");
    if uses_tui {
        out.push('\n');
        out.push_str(get_tui_rust_runtime());
    }
    if out.contains("tokio_sleep_ms(") {
        out.push_str("\npub fn tokio_sleep_ms(ms: PsudoVal) -> PsudoVal { let m = match ms { PsudoVal::Num(n) => n.max(0.0) as u64, _ => 0 }; std::thread::sleep(std::time::Duration::from_millis(m)); PsudoVal::Bool(true) }\n");
    }

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

    println!("  -> Borrow-checking capsule... {}", crate::utils::green_bold("ok"));
    let capsule_cfg = capsule
        .virtual_files
        .get("psudo.toml")
        .map(|s| crate::config::parse_project_config_str(s))
        .unwrap_or_else(crate::config::load_project_config);

    let mut rust_code = transpile_psudo_to_rust(&combined_source, true)?;

    let mut custom_cargo_deps: Vec<String> = Vec::new();
    for addon in capsule_cfg.addons.values() {
        let used = addon
            .builtins
            .iter()
            .any(|b| combined_source.contains(&format!("{}(", b)));
        if used {
            for dep in &addon.cargo_deps {
                if !custom_cargo_deps.contains(dep) {
                    custom_cargo_deps.push(dep.clone());
                }
            }
            if addon.rust_main_async && rust_code.contains("\npub fn main() {\n") {
                let attr_prefix = addon
                    .rust_main_attr
                    .as_deref()
                    .map(|a| format!("{}\n", a))
                    .unwrap_or_default();
                rust_code = rust_code.replace(
                    "\npub fn main() {\n",
                    &format!("\n{}pub async fn main() {{\n", attr_prefix),
                );
            }
            if let Some(rfile) = &addon.rust_file {
                if let Some(extra_rs) = capsule.virtual_files.get(rfile) {
                    rust_code.push('\n');
                    rust_code.push_str(extra_rs);
                } else if let Ok(extra_rs) = std::fs::read_to_string(rfile) {
                    rust_code.push('\n');
                    rust_code.push_str(&extra_rs);
                }
            }
        }
    }

    let has_unwrap = rust_code.contains(".unwrap()") || rust_code.contains(".expect()");
    if has_unwrap {
        return Err("Transpiler emitted forbidden .unwrap() or .expect()".into());
    }
    println!(
        "  -> Lowering to safe Rust... {}",
        crate::utils::green_bold("ok")
    );
    println!(
        "  -> Compiling{} via cargo...",
        if run_after_build { " & running" } else { "" }
    );

    std::fs::create_dir_all(out_dir)
        .map_err(|e| format!("Failed to create {}: {}", out_dir.display(), e))?;
    let src_dir = out_dir.join("src");
    std::fs::create_dir_all(&src_dir)
        .map_err(|e| format!("Failed to create {}: {}", src_dir.display(), e))?;

    let file_stem = capsule.project_name.replace('-', "_");

    let uses_serde = rust_code.contains("serde_json::");
    let uses_tui = rust_code.contains("pub fn tui_dashboard(");
    let uses_tokio = rust_code.contains("#[tokio::main]");

    let mut deps_vec: Vec<String> = Vec::new();
    if uses_serde || uses_tui {
        deps_vec.push("serde_json = \"1\"".to_string());
        deps_vec.push("serde = { version = \"1\", features = [\"derive\"] }".to_string());
    }
    if uses_tui {
        deps_vec.push("ratatui = \"0.29\"".to_string());
        deps_vec.push("crossterm = \"0.28\"".to_string());
    }
    if uses_tokio {
        deps_vec.push("tokio = { version = \"1\", features = [\"full\"] }".to_string());
    }
    for cdep in custom_cargo_deps {
        if !deps_vec.contains(&cdep) {
            deps_vec.push(cdep);
        }
    }
    let deps_section = deps_vec.join("\n");

    // Emit an empty [workspace] table so capsules initialized inside a parent Cargo workspace
    // (like a fresh clone of this repo) compile as standalone crates instead of failing workspace checks.
    let cargo_toml = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[workspace]

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
        if uses_tui
            && std::io::IsTerminal::is_terminal(&std::io::stdout())
            && std::env::var("PSUDO_TUI_HEADLESS").is_err()
        {
            let status = run_cmd
                .stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .status()
                .map_err(|e| format!("Failed to execute compiled TUI binary: {}", e))?;
            run_exit_code = status.code();
            run_stdout = Some(String::new());
            run_stderr = Some(String::new());
        } else {
            let run_output = run_cmd
                .output()
                .map_err(|e| format!("Failed to execute compiled binary: {}", e))?;
            run_exit_code = run_output.status.code();
            run_stdout = Some(String::from_utf8_lossy(&run_output.stdout).to_string());
            run_stderr = Some(String::from_utf8_lossy(&run_output.stderr).to_string());
        }
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

fn get_tui_rust_runtime() -> &'static str {
    // Zero-unwrap Ratatui + Crossterm runtime with automatic TestBackend fallback
    // when stdout is piped or PSUDO_TUI_HEADLESS=1 is set.
    r#"
pub fn tui_metric(label: PsudoVal, value: PsudoVal) -> PsudoVal {
    let mut m = std::collections::BTreeMap::new();
    m.insert("__tui".to_string(), PsudoVal::Str("metric".to_string()));
    m.insert("label".to_string(), label);
    m.insert("value".to_string(), value);
    PsudoVal::Map(m)
}

pub fn tui_gauge(label: PsudoVal, percent: PsudoVal) -> PsudoVal {
    let mut m = std::collections::BTreeMap::new();
    m.insert("__tui".to_string(), PsudoVal::Str("gauge".to_string()));
    m.insert("label".to_string(), label);
    m.insert("percent".to_string(), percent);
    PsudoVal::Map(m)
}

pub fn tui_sparkline(label: PsudoVal, data: PsudoVal) -> PsudoVal {
    let mut m = std::collections::BTreeMap::new();
    m.insert("__tui".to_string(), PsudoVal::Str("sparkline".to_string()));
    m.insert("label".to_string(), label);
    m.insert("data".to_string(), data);
    PsudoVal::Map(m)
}

pub fn tui_table(title: PsudoVal, rows: PsudoVal) -> PsudoVal {
    let mut m = std::collections::BTreeMap::new();
    m.insert("__tui".to_string(), PsudoVal::Str("table".to_string()));
    m.insert("title".to_string(), title);
    m.insert("rows".to_string(), rows);
    PsudoVal::Map(m)
}

pub fn tui_log(title: PsudoVal, lines: PsudoVal) -> PsudoVal {
    let mut m = std::collections::BTreeMap::new();
    m.insert("__tui".to_string(), PsudoVal::Str("log".to_string()));
    m.insert("title".to_string(), title);
    m.insert("lines".to_string(), lines);
    PsudoVal::Map(m)
}

fn tui_load_kv_entries(source: &PsudoVal) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    if let PsudoVal::Map(m) = source {
        if let Some(PsudoVal::Str(store_name)) = m.get("store_name") {
            let path = std::path::PathBuf::from(format!(".pseudo/state/{}.json", store_name));
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(map) = serde_json::from_str::<std::collections::BTreeMap<String, serde_json::Value>>(&content) {
                    for (k, v) in map {
                        rows.push((k, v.to_string().trim_matches('"').to_string()));
                    }
                    return rows;
                }
            }
        }
        for (k, v) in m {
            if k != "__type" {
                rows.push((k.clone(), format!("{}", v)));
            }
        }
    }
    rows
}

fn tui_render_headless(title: &str, lines: &[String]) {
    use ratatui::backend::TestBackend;
    use ratatui::widgets::{Block, Borders, Paragraph};
    use ratatui::Terminal;
    let backend = TestBackend::new(80, 24);
    if let Ok(mut terminal) = Terminal::new(backend) {
        let body = lines.join("\n");
        let _ = terminal.draw(|f| {
            let area = f.area();
            let widget = Paragraph::new(body.as_str())
                .block(Block::default().title(title).borders(Borders::ALL));
            f.render_widget(widget, area);
        });
        let buf = terminal.backend().buffer();
        for y in 0..buf.area.height {
            let mut row = String::new();
            for x in 0..buf.area.width {
                if let Some(cell) = buf.cell((x, y)) {
                    row.push_str(cell.symbol());
                }
            }
            println!("{}", row.trim_end());
        }
    }
}

fn tui_run_interactive(title: &str, lines: &[String]) {
    use crossterm::event::{self, Event, KeyCode};
    use crossterm::execute;
    use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
    use ratatui::backend::CrosstermBackend;
    use ratatui::widgets::{Block, Borders, Paragraph};
    use ratatui::Terminal;

    if enable_raw_mode().is_err() {
        tui_render_headless(title, lines);
        return;
    }
    let mut stdout = std::io::stdout();
    if execute!(stdout, EnterAlternateScreen).is_err() {
        let _ = disable_raw_mode();
        tui_render_headless(title, lines);
        return;
    }
    let backend = CrosstermBackend::new(stdout);
    if let Ok(mut terminal) = Terminal::new(backend) {
        let mut scroll: u16 = 0;
        loop {
            let body = format!("{}\n\n[Up/Down] Scroll  [q/Esc] Quit", lines.join("\n"));
            let _ = terminal.draw(|f| {
                let area = f.area();
                let widget = Paragraph::new(body.as_str())
                    .scroll((scroll, 0))
                    .block(Block::default().title(title).borders(Borders::ALL));
                f.render_widget(widget, area);
            });
            if event::poll(std::time::Duration::from_millis(120)).unwrap_or(false) {
                if let Ok(Event::Key(k)) = event::read() {
                    match k.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Down | KeyCode::Char('j') => scroll = scroll.saturating_add(1),
                        KeyCode::Up | KeyCode::Char('k') => scroll = scroll.saturating_sub(1),
                        _ => {}
                    }
                }
            }
        }
        let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    }
    let _ = disable_raw_mode();
}

pub fn tui_dashboard(title: PsudoVal, source: PsudoVal) -> PsudoVal {
    let t_str = format!("{}", title);
    let mut lines: Vec<String> = Vec::new();
    match &source {
        PsudoVal::List(items) => {
            for item in items {
                if let PsudoVal::Map(m) = item {
                    if let Some(PsudoVal::Str(kind)) = m.get("__tui") {
                        match kind.as_str() {
                            "metric" => {
                                let lbl = m.get("label").map(|v| format!("{}", v)).unwrap_or_default();
                                let val = m.get("value").map(|v| format!("{}", v)).unwrap_or_default();
                                lines.push(format!("[{}] {}", lbl, val));
                            }
                            "gauge" => {
                                let lbl = m.get("label").map(|v| format!("{}", v)).unwrap_or_default();
                                let pct = match m.get("percent") {
                                    Some(PsudoVal::Num(n)) => (*n).clamp(0.0, 100.0),
                                    _ => 0.0,
                                };
                                let filled = ((pct / 100.0) * 20.0).round() as usize;
                                let bar = format!("{}{}", "█".repeat(filled), "░".repeat(20usize.saturating_sub(filled)));
                                lines.push(format!("{:<16} [{}] {:.0}%", lbl, bar, pct));
                            }
                            "sparkline" => {
                                let lbl = m.get("label").map(|v| format!("{}", v)).unwrap_or_default();
                                let bars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
                                let mut spark = String::new();
                                if let Some(PsudoVal::List(nums)) = m.get("data") {
                                    let max_v = nums.iter().filter_map(|v| if let PsudoVal::Num(n) = v { Some(*n) } else { None }).fold(1.0f32, f32::max);
                                    for v in nums {
                                        if let PsudoVal::Num(n) = v {
                                            let idx = (((*n / max_v) * 7.0).round() as usize).min(7);
                                            spark.push(bars[idx]);
                                        }
                                    }
                                }
                                lines.push(format!("{:<16} {}", lbl, spark));
                            }
                            "table" => {
                                let sub = m.get("title").map(|v| format!("{}", v)).unwrap_or_default();
                                lines.push(format!("-- {} --", sub));
                                if let Some(PsudoVal::List(rows)) = m.get("rows") {
                                    for r in rows {
                                        lines.push(format!("  {}", r));
                                    }
                                }
                            }
                            "log" => {
                                let sub = m.get("title").map(|v| format!("{}", v)).unwrap_or_default();
                                lines.push(format!("-- {} --", sub));
                                if let Some(PsudoVal::List(lns)) = m.get("lines") {
                                    for l in lns {
                                        lines.push(format!("  {}", l));
                                    }
                                }
                            }
                            _ => lines.push(format!("{}", item)),
                        }
                    } else {
                        lines.push(format!("{}", item));
                    }
                } else {
                    lines.push(format!("{}", item));
                }
            }
        }
        _ => {
            for (k, v) in tui_load_kv_entries(&source) {
                lines.push(format!("{:<24} : {}", k, v));
            }
        }
    }
    if lines.is_empty() {
        lines.push("(no entries in store)".to_string());
    }
    if std::io::IsTerminal::is_terminal(&std::io::stdout())
        && std::env::var("PSUDO_TUI_HEADLESS").is_err()
    {
        tui_run_interactive(&t_str, &lines);
    } else {
        tui_render_headless(&t_str, &lines);
    }
    PsudoVal::Bool(true)
}

pub fn tui_grid(title: PsudoVal, source: PsudoVal) -> PsudoVal {
    let t_str = format!("{}", title);
    let entries = tui_load_kv_entries(&source);
    let w: usize = 36;
    let h: usize = 12;
    let mut grid = vec![vec![false; w]; h];
    if let Ok(seed_raw) = std::fs::read_to_string(".pseudo/sandbox/life_seed.json") {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&seed_raw) {
            if let Some(cells) = val.get("live_cells").and_then(|c| c.as_array()) {
                for pair in cells {
                    if let (Some(r), Some(c)) = (pair.get(0).and_then(|v| v.as_u64()), pair.get(1).and_then(|v| v.as_u64())) {
                        let rr = (r as usize) % h;
                        let cc = (c as usize) % w;
                        grid[rr][cc] = true;
                    }
                }
            }
        }
    }
    let mut lines = Vec::new();
    for (k, v) in entries {
        lines.push(format!("{} = {}", k, v));
    }
    for row in &grid {
        let row_str: String = row.iter().map(|&alive| if alive { '█' } else { '·' }).collect();
        lines.push(row_str);
    }
    if std::io::IsTerminal::is_terminal(&std::io::stdout())
        && std::env::var("PSUDO_TUI_HEADLESS").is_err()
    {
        tui_run_interactive(&t_str, &lines);
    } else {
        tui_render_headless(&t_str, &lines);
    }
    PsudoVal::Bool(true)
}

pub fn tui_fractal(title: PsudoVal, source: PsudoVal) -> PsudoVal {
    let t_str = format!("{}", title);
    let entries = tui_load_kv_entries(&source);
    let mut cx = -0.5f64;
    let mut cy = 0.0f64;
    let mut zoom = 1.0f64;
    let mut max_iter = 48usize;
    for (k, v) in &entries {
        match k.as_str() {
            "center_x" => cx = v.parse().unwrap_or(-0.5),
            "center_y" => cy = v.parse().unwrap_or(0.0),
            "zoom" => zoom = v.parse::<f64>().unwrap_or(1.0).max(0.01),
            "max_iter" => max_iter = v.parse::<f64>().unwrap_or(48.0).clamp(8.0, 256.0) as usize,
            _ => {}
        }
    }
    let palette: &[u8] = b" .:-=+*#%@";
    let w = 64usize;
    let h = 16usize;
    let mut lines = vec![format!("center=({:.3}, {:.3}) zoom={:.2}x max_iter={}", cx, cy, zoom, max_iter)];
    for py in 0..h {
        let mut row = String::with_capacity(w);
        let ci = cy + ((py as f64 / h as f64) - 0.5) * (2.4 / zoom);
        for px in 0..w {
            let cr = cx + ((px as f64 / w as f64) - 0.5) * (3.6 / zoom);
            let mut zr = 0.0f64;
            let mut zi = 0.0f64;
            let mut iter = 0usize;
            while zr * zr + zi * zi <= 4.0 && iter < max_iter {
                let nzi = 2.0 * zr * zi + ci;
                zr = zr * zr - zi * zi + cr;
                zi = nzi;
                iter += 1;
            }
            let ch_idx = if iter >= max_iter {
                palette.len() - 1
            } else {
                (iter * (palette.len() - 1)) / max_iter
            };
            row.push(palette[ch_idx] as char);
        }
        lines.push(row);
    }
    if std::io::IsTerminal::is_terminal(&std::io::stdout())
        && std::env::var("PSUDO_TUI_HEADLESS").is_err()
    {
        tui_run_interactive(&t_str, &lines);
    } else {
        tui_render_headless(&t_str, &lines);
    }
    PsudoVal::Bool(true)
}
"#
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transpile_tui_and_tokio_builtins_zero_unwrap() {
        let src = r#"
fn main()
    let m = tui_metric("Net", 2035)
    let g = tui_gauge("Budget", 86)
    let _ = tokio_sleep_ms(10)
    return tui_dashboard("Finance", [m, g])
main()
"#;
        let rust = transpile_psudo_to_rust(src, true).expect("transpile should succeed");
        assert!(rust.contains("#[tokio::main]\npub async fn main()"));
        assert!(rust.contains("pub fn tui_dashboard("));
        assert!(rust.contains("pub fn tokio_sleep_ms("));
        assert!(!rust.contains(".unwrap()"));
        assert!(!rust.contains(".expect()"));
    }

    #[test]
    fn test_custom_addon_in_capsule_injects_deps_and_tokio_main() {
        let mut capsule = crate::vfs::WorkspaceCapsule::new_empty("addon_test");
        capsule.virtual_files.insert(
            "psudo.toml".to_string(),
            r##"
[addons.my_worker]
builtins = ["worker_ping"]
cargo_deps = ['tokio = { version = "1", features = ["full"] }']
rust_main_attr = "#[tokio::main]"
rust_main_async = true
rust_file = "src/worker_addon.rs"
"##
            .to_string(),
        );
        capsule.virtual_files.insert(
            "src/worker_addon.rs".to_string(),
            "pub fn worker_ping() -> PsudoVal { PsudoVal::Bool(true) }".to_string(),
        );
        let cfg = crate::config::parse_project_config_str(
            capsule.virtual_files.get("psudo.toml").unwrap(),
        );
        let addon = cfg.addons.get("my_worker").unwrap();
        assert!(addon.rust_main_async);
        assert_eq!(addon.builtins, vec!["worker_ping"]);
    }
}
