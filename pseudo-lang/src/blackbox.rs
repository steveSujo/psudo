use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::context::{analyze_psudo_source, SymbolContextDigest};
use crate::lexer::Lexer;
use crate::ownership::{BorrowChecker, CheckedEnvironment};
use crate::parser::{CheckedInterpret, Parser};
use crate::pir_parser::parse_pir;
use crate::resolver::Resolver;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlackBoxSynthRequest {
    pub module_path: String,
    pub target_symbol: Option<String>,
    pub directive: Option<String>,
    pub model_path: Option<PathBuf>,
}

impl Default for BlackBoxSynthRequest {
    fn default() -> Self {
        Self {
            module_path: "src/main.psudo".into(),
            target_symbol: None,
            directive: None,
            model_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlackBoxSynthResult {
    pub symbol: String,
    pub synthesized_psudo: String,
    pub verified_borrows: bool,
    pub engine_used: String,
    pub diagnostics: Vec<String>,
    #[serde(default)]
    pub directive: String,
}

pub fn synthesize_stubs_in_source(
    source: &str,
    req: &BlackBoxSynthRequest,
) -> Result<(String, Vec<BlackBoxSynthResult>), String> {
    let digest = analyze_psudo_source(source, &req.module_path, None)?;
    let mut modified_source = source.to_string();
    let mut results = Vec::new();

    let target_symbols: Vec<SymbolContextDigest> = digest
        .symbols
        .into_iter()
        .filter(|s| {
            if let Some(target) = &req.target_symbol {
                &s.name == target
            } else {
                s.is_stub
            }
        })
        .collect();

    for sym in target_symbols {
        let raw_directive = req
            .directive
            .as_deref()
            .or(sym.stub_directive.as_deref())
            .unwrap_or("implement function");
        let directive = normalize_directive(raw_directive);

        let (synthesized_body, engine_name) =
            try_synthesize_symbol(&sym, &directive, req.model_path.as_deref())?;

        if let Some(start) = synthesized_body.find("<clarify>") {
            let end = synthesized_body.find("</clarify>").unwrap_or(synthesized_body.len());
            let msg = synthesized_body[start + 9..end].trim();
            return Err(format!("Clarification required for '{}': {}", sym.name, msg));
        }

        let full_fn = format!(
            "fn {}({})\n{}",
            sym.name,
            sym.params
                .iter()
                .map(|(p, _)| p.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            indent_code(&synthesized_body, 4)
        );

        // Verify borrow checker and resolver diagnostics on synthesized function
        let verified_borrows = verify_psudo_borrows(&full_fn);
        let diagnostics = match analyze_psudo_source(&full_fn, &req.module_path, None) {
            Ok(d) => d.diagnostics,
            Err(e) => vec![e],
        };

        // Splice in-place into source
        modified_source = splice_function_in_source(&modified_source, &sym.name, &full_fn);

        results.push(BlackBoxSynthResult {
            symbol: sym.name.clone(),
            synthesized_psudo: full_fn,
            verified_borrows,
            engine_used: engine_name,
            diagnostics,
            directive: raw_directive.to_string(),
        });
    }

    Ok((modified_source, results))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DaemonHealthResponse {
    pub status: String,
    pub model: String,
    pub idle_remaining_s: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DaemonSynthRequest {
    pub symbol: String,
    pub params: Vec<String>,
    pub directive: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DaemonSynthResponse {
    pub symbol: String,
    pub psudo: String,
    #[serde(default)]
    pub pir: String,
    pub decode_tps: f32,
}

pub fn daemon_port() -> u16 {
    std::env::var("PSUDO_DAEMON_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(11434)
}

pub fn stop_daemon(port: u16) -> Result<bool, String> {
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpStream};
    use std::time::Duration;

    let addr: SocketAddr = format!("127.0.0.1:{}", port)
        .parse()
        .map_err(|e| format!("Invalid daemon address: {}", e))?;
    let mut stream = match TcpStream::connect_timeout(&addr, Duration::from_millis(500)) {
        Ok(s) => s,
        Err(_) => return Ok(false),
    };
    let req = format!(
        "POST /shutdown HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        port
    );
    let _ = stream.write_all(req.as_bytes());
    let mut resp = Vec::new();
    let _ = stream.read_to_end(&mut resp);
    let _ = std::fs::remove_file(".pseudo/state/daemon.mtime");
    Ok(true)
}

fn binary_mtime_secs(path: &Path) -> Option<u64> {
    std::fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

pub fn get_daemon_health(port: u16) -> Result<DaemonHealthResponse, String> {
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpStream};
    use std::time::Duration;

    let addr: SocketAddr = format!("127.0.0.1:{}", port)
        .parse()
        .map_err(|e| format!("Invalid daemon address: {}", e))?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(500))
        .map_err(|e| format!("Daemon connection failed: {}", e))?;

    stream.set_read_timeout(Some(Duration::from_secs(60))).ok();
    let req = format!(
        "GET /health HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
        port
    );
    stream
        .write_all(req.as_bytes())
        .map_err(|e| e.to_string())?;

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|e| e.to_string())?;

    let resp_str = String::from_utf8_lossy(&response);
    let body_start = resp_str.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    let body = &resp_str[body_start..];

    serde_json::from_str(body).map_err(|e| format!("Failed to parse health response: {}", e))
}

// Cold-loading the GGUF model on every @stub kills throughput; keeping a local HTTP daemon
// up lets me keep the weights mmap'd across calls so iterative synthesis stays snappy.
pub fn ensure_daemon_running(model_override: Option<&Path>) -> Result<u16, String> {
    let port = daemon_port();

    let engine_bin = match find_engine_binary() {
        Some(b) if b.exists() => b,
        Some(b) => {
            return Err(format!(
                "Internal error: 'pseudo-engine' binary at '{}' does not exist. Run 'cargo build --release' in the repository root.",
                b.display()
            ));
        }
        None => {
            return Err(
                "Internal error: 'pseudo-engine' binary not found. Run 'cargo build --release' in the repository root (or 'cargo build --release -p pseudo-engine')."
                    .to_string(),
            );
        }
    };

    let current_mtime = binary_mtime_secs(&engine_bin).unwrap_or(0);
    let mtime_file = std::path::Path::new(".pseudo/state/daemon.mtime");
    if get_daemon_health(port).is_ok() {
        let recorded_mtime = std::fs::read_to_string(mtime_file)
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(current_mtime);
        // Restart stale background daemons automatically after `cargo build` updates `pseudo-engine`.
        if current_mtime > recorded_mtime {
            eprintln!("[daemon] Newer pseudo-engine binary detected; restarting daemon...");
            let _ = stop_daemon(port);
            std::thread::sleep(std::time::Duration::from_millis(200));
        } else {
            return Ok(port);
        }
    }

    let resolved_model = model_override
        .map(|p| p.to_path_buf())
        .or_else(find_model_path);

    let model = match resolved_model {
        Some(m) if m.exists() => m,
        Some(m) => {
            return Err(format!(
                "Internal error: GGUF model '{}' does not exist.",
                m.display()
            ));
        }
        None => {
            return Err(
                "Internal error: GGUF model 'pseudo-33m-tq2_0.gguf' is not loaded. Ensure model exists or set PSUDO_MODEL_PATH."
                    .to_string(),
            );
        }
    };

    let log_dir = std::path::Path::new(".pseudo/state");
    let _ = std::fs::create_dir_all(log_dir);
    let log_file_path = log_dir.join("daemon.log");
    let log_file = std::fs::File::create(&log_file_path)
        .map_err(|e| format!("Failed to create daemon log file: {}", e))?;
    let log_file_stderr = log_file
        .try_clone()
        .map_err(|e| format!("Failed to clone log file handle: {}", e))?;

    let mut child = std::process::Command::new(&engine_bin)
        .arg("--daemon")
        .arg("-m")
        .arg(&model)
        .arg("--port")
        .arg(port.to_string())
        // Detach stdin so PTY recorders (asciinema/vhs) don't wait on daemon EOF.
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::from(log_file))
        .stderr(std::process::Stdio::from(log_file_stderr))
        .spawn()
        .map_err(|e| format!("Internal error: Failed to spawn daemon: {}", e))?;
    let child_pid = child.id();

    for _ in 0..50 {
        if let Ok(Some(status)) = child.try_wait() {
            let stderr_content = std::fs::read_to_string(&log_file_path).unwrap_or_default();
            return Err(format!(
                "Internal error: pseudo-engine daemon exited prematurely (status {:?}): {}",
                status, stderr_content
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        if get_daemon_health(port).is_ok() {
            let _ = std::fs::write(mtime_file, current_mtime.to_string());
            eprintln!(
                "{}",
                crate::utils::yellow_err(&format!(
                    "[daemon] Started background pseudo-engine (PID {}). Stop anytime with 'psudo daemon stop'.",
                    child_pid
                ))
            );
            return Ok(port);
        }
    }

    let stderr_content = std::fs::read_to_string(&log_file_path).unwrap_or_default();
    Err(format!(
        "Internal error: Spawned pseudo-engine daemon on port {} failed to become healthy within 5 seconds. Log output:\n{}",
        port, stderr_content
    ))
}

pub fn post_daemon_synth(
    port: u16,
    req: &DaemonSynthRequest,
) -> Result<DaemonSynthResponse, String> {
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpStream};
    use std::time::Duration;

    let addr: SocketAddr = format!("127.0.0.1:{}", port)
        .parse()
        .map_err(|e| format!("Invalid daemon address: {}", e))?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(5))
        .map_err(|e| format!("Daemon connection failed: {}", e))?;

    let json_body = serde_json::to_string(req).map_err(|e| e.to_string())?;
    let http_req = format!(
        "POST /synth HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        port,
        json_body.len(),
        json_body
    );

    stream
        .write_all(http_req.as_bytes())
        .map_err(|e| e.to_string())?;

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|e| e.to_string())?;

    let resp_str = String::from_utf8_lossy(&response);
    let body_start = resp_str.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    let body = &resp_str[body_start..];

    serde_json::from_str(body).map_err(|e| format!("Failed to parse daemon synth response: {}", e))
}

fn extract_body_from_psudo(psudo_str: &str, fn_name: &str) -> String {
    let lines: Vec<&str> = psudo_str.lines().collect();
    let mut body_lines = Vec::new();
    let mut in_body = false;

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.starts_with(&format!("fn {}(", fn_name))
            || trimmed.starts_with(&format!("fn {} (", fn_name))
            || trimmed.starts_with("fn ")
        {
            in_body = true;
            continue;
        }
        if in_body {
            body_lines.push(line.to_string());
        }
    }

    if body_lines.is_empty() {
        psudo_str.to_string()
    } else {
        let min_indent = body_lines
            .iter()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.len() - l.trim_start().len())
            .min()
            .unwrap_or(0);
        body_lines
            .into_iter()
            .map(|l| {
                if l.trim().is_empty() {
                    String::new()
                } else {
                    l.chars().skip(min_indent).collect()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn try_synthesize_symbol(
    sym: &SymbolContextDigest,
    directive: &str,
    model_override: Option<&Path>,
) -> Result<(String, String), String> {
    let param_names_vec: Vec<String> = sym.params.iter().map(|(p, _)| p.clone()).collect();

    match ensure_daemon_running(model_override) {
        Ok(port) => {
            let synth_req = DaemonSynthRequest {
                symbol: sym.name.clone(),
                params: param_names_vec,
                directive: Some(directive.to_string()),
            };
            match post_daemon_synth(port, &synth_req) {
                Ok(resp) => {
                    let body = extract_body_from_psudo(&resp.psudo, &sym.name);
                    Ok((body, "gguf+daemon".to_string()))
                }
                Err(e) => Err(format!(
                    "Internal error: Daemon synthesis failed for symbol '{}': {}",
                    sym.name, e
                )),
            }
        }
        Err(daemon_err) => {
            let engine_bin = find_engine_binary();
            let model_path = model_override
                .map(|p| p.to_path_buf())
                .or_else(find_model_path);

            if let (Some(bin), Some(model)) = (&engine_bin, &model_path) {
                if bin.exists() && model.exists() {
                    let param_names = param_names_vec.join(" ");
                    let pir_prompt = format!("f {} ({}) [\n", sym.name, param_names);
                    if let Ok(output) = run_engine_subprocess(bin, model, &pir_prompt, 0.0, 64) {
                        let full_pir = format!("{}{}", pir_prompt, output);
                        if let Ok(stmts) = parse_pir(&full_pir) {
                            if verify_pir_statements(&stmts) {
                                if let Some(psudo) = decompile_pir_body(&full_pir) {
                                    return Ok((psudo, "gguf+pda".to_string()));
                                }
                            }
                        }
                    }
                }
            }

            Err(daemon_err)
        }
    }
}

fn decompile_pir_body(pir_source: &str) -> Option<String> {
    let lines: Vec<&str> = pir_source.lines().collect();
    let mut body_lines = Vec::new();
    let mut in_body = false;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with("f ") && trimmed.contains('[') {
            in_body = true;
            continue;
        }
        if in_body {
            if trimmed == "]" || trimmed.ends_with(']') {
                break;
            }
            if !trimmed.is_empty() {
                // simple polish decompile: r (+ a b) -> return a + b
                if let Some(rest) = trimmed.strip_prefix("r ") {
                    let decompiled_expr = decompile_polish_expr(rest);
                    body_lines.push(format!("return {}", decompiled_expr));
                } else if let Some(rest) = trimmed.strip_prefix("v ") {
                    let mut parts = rest.split_whitespace();
                    if let (Some(var), Some(_expr_part)) = (parts.next(), parts.next()) {
                        let remainder = &rest[var.len() + 1..].trim();
                        body_lines.push(format!(
                            "let {} = {}",
                            var,
                            decompile_polish_expr(remainder)
                        ));
                    }
                } else {
                    body_lines.push(trimmed.to_string());
                }
            }
        }
    }

    if body_lines.is_empty() {
        None
    } else {
        Some(body_lines.join("\n"))
    }
}

fn decompile_polish_expr(expr: &str) -> String {
    let trimmed = expr.trim();
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        let inner = &trimmed[1..trimmed.len() - 1].trim();
        let parts: Vec<&str> = inner.split_whitespace().collect();
        if parts.len() >= 3 && matches!(parts[0], "+" | "-" | "*" | "/" | "==" | "!=" | "<" | ">") {
            let op = parts[0];
            let left = parts[1];
            let right = parts[2..].join(" ");
            return format!("{} {} {}", left, op, right);
        }
    }
    trimmed.to_string()
}

fn verify_pir_statements(stmts: &[crate::parser::Statment]) -> bool {
    let borrow_checker = Rc::new(RefCell::new(BorrowChecker::new()));
    let mut checked_interp = CheckedInterpret {
        environment: Rc::new(RefCell::new(CheckedEnvironment::new(borrow_checker))),
        check_borrows: true,
    };
    for s in stmts {
        checked_interp.execute(s);
    }
    let ok = checked_interp.environment.borrow().errors.is_empty();
    ok
}

fn verify_psudo_borrows(psudo_source: &str) -> bool {
    let mut lexer = Lexer::new(psudo_source.to_string());
    if let Ok(tokens) = lexer.scan_tokens() {
        let mut parser = Parser::new(tokens);
        if let Ok(stmts) = parser.parse() {
            let resolver = Resolver::new();
            let res = resolver.resolve(&stmts);
            if !res.errors.is_empty() {
                return false;
            }
            return verify_pir_statements(&stmts);
        }
    }
    false
}

fn splice_function_in_source(source: &str, fn_name: &str, replacement: &str) -> String {
    let mut lines = source.lines().collect::<Vec<_>>();
    let mut start_idx = None;
    let mut end_idx = None;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with(&format!("fn {}(", fn_name))
            || trimmed.starts_with(&format!("fn {} (", fn_name))
        {
            start_idx = Some(i);
            continue;
        }
        if start_idx.is_some() && end_idx.is_none() {
            // Check for next unindented top-level statement or EOF
            if !line.starts_with(' ') && !line.starts_with('\t') && !trimmed.is_empty() {
                end_idx = Some(i);
                break;
            }
        }
    }

    if let Some(start) = start_idx {
        let end = end_idx.unwrap_or(lines.len());
        lines.splice(start..end, replacement.lines());
        lines.join("\n")
    } else {
        format!("{}\n\n{}", source.trim_end(), replacement)
    }
}

fn indent_code(code: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    code.lines()
        .map(|l| {
            if l.trim().is_empty() {
                String::new()
            } else {
                format!("{}{}", prefix, l)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn find_engine_binary() -> Option<PathBuf> {
    crate::mcp::find_engine_binary()
}

fn find_model_path() -> Option<PathBuf> {
    crate::mcp::resolve_model_path(None)
}

fn run_engine_subprocess(
    bin: &Path,
    model: &Path,
    prompt: &str,
    temp: f32,
    max_tokens: usize,
) -> Result<String, String> {
    let output = std::process::Command::new(bin)
        .arg("-m")
        .arg(model)
        .arg("-p")
        .arg(prompt)
        .arg("--temp")
        .arg(temp.to_string())
        .arg("--max-tokens")
        .arg(max_tokens.to_string())
        .output()
        .map_err(|e| format!("Subprocess launch failed: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Engine failed (code {:?}): {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Some(pos) = stdout.find("Output:") {
        let after = &stdout[pos + 7..];
        let clean = after
            .split("\n\n.psudo Decompiled View")
            .next()
            .unwrap_or(after);
        let end = clean.find("tps |").unwrap_or(clean.len());
        Ok(clean[..end].trim().to_string())
    } else {
        Ok(stdout.trim().to_string())
    }
}
pub fn synthesize_stubs_in_capsule(
    capsule: &mut crate::vfs::WorkspaceCapsule,
    target_symbol: Option<&str>,
    directive: Option<&str>,
    model_path: Option<&Path>,
) -> Result<Vec<BlackBoxSynthResult>, String> {
    let mut all_results = Vec::new();
    let mut modules: Vec<(String, String)> = capsule
        .virtual_files
        .iter()
        .filter(|(p, _)| p.ends_with(".psudo"))
        .map(|(p, s)| (p.clone(), s.clone()))
        .collect();
    modules.sort_by(|a, b| a.0.cmp(&b.0));

    for (vpath, source) in modules {
        let req = BlackBoxSynthRequest {
            module_path: vpath.clone(),
            target_symbol: target_symbol.map(ToString::to_string),
            directive: directive.map(ToString::to_string),
            model_path: model_path.map(|p| p.to_path_buf()),
        };

        let (synthesized_source, results) = synthesize_stubs_in_source(&source, &req)?;
        if !results.is_empty() {
            capsule.set_file(&vpath, synthesized_source)?;
            for r in &results {
                let dir_str = directive.unwrap_or(r.directive.as_str());
                capsule.record_stub_trace(&r.symbol, "SYNTHESIZED", dir_str);
            }
            all_results.extend(results);
        }
    }
    Ok(all_results)
}

fn normalize_directive(raw: &str) -> String {
    raw.split_whitespace()
        .map(|tok| match tok.to_lowercase().as_str() {
            "fibonaci" | "fibonnaci" => "fibonacci",
            _ => tok,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_mock_fail_fast_on_missing_model() {
        let source = "fn fib(n)\n    pass";
        let req = BlackBoxSynthRequest {
            module_path: "src/main.psudo".to_string(),
            target_symbol: Some("fib".to_string()),
            directive: Some("compute fibonacci".to_string()),
            model_path: Some(PathBuf::from("non_existent_model.gguf")),
        };

        let res = synthesize_stubs_in_source(source, &req);
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(
            err.contains("Internal error")
                || err.contains("does not exist")
                || err.contains("model")
        );
    }
}
