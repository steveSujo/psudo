use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::engine::Engine;
use crate::pir_decompiler;
use crate::sampler::{Sampler, SparseChannel};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub model: String,
    pub idle_remaining_s: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SynthRequest {
    pub symbol: String,
    #[serde(default)]
    pub params: Vec<String>,
    #[serde(default)]
    pub directive: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthResponse {
    pub symbol: String,
    pub psudo: String,
    pub decode_tps: f32,
}

pub fn run_daemon(
    mut engine: Engine,
    port: u16,
    idle_timeout_secs: u64,
    model_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr)?;
    listener.set_nonblocking(true)?;
    eprintln!("[pseudo-engine-daemon] Server listening on http://{}", addr);

    let last_activity = Arc::new(Mutex::new(Instant::now()));

    loop {
        let elapsed = last_activity.lock().unwrap().elapsed().as_secs();
        if elapsed >= idle_timeout_secs {
            eprintln!(
                "[pseudo-engine-daemon] Idle timeout ({}s) reached. Shutting down.",
                idle_timeout_secs
            );
            break;
        }

        match listener.accept() {
            Ok((mut stream, _)) => {
                let _ = stream.set_nonblocking(false);
                let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));
                *last_activity.lock().unwrap() = Instant::now();
                let remaining = idle_timeout_secs
                    .saturating_sub(last_activity.lock().unwrap().elapsed().as_secs());

                if let Err(e) = handle_client(&mut stream, &mut engine, &model_path, remaining) {
                    eprintln!("[pseudo-engine-daemon] Client error: {}", e);
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                eprintln!("[pseudo-engine-daemon] Accept error: {}", e);
            }
        }
    }

    Ok(())
}

fn handle_client(
    stream: &mut TcpStream,
    engine: &mut Engine,
    model_path: &Path,
    idle_remaining_s: u64,
) -> Result<bool, Box<dyn std::error::Error>> {
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    let mut buffer = Vec::new();
    let mut temp_buf = [0u8; 1024];
    let mut header_end = None;
    let mut content_length = None;

    loop {
        let n = match stream.read(&mut temp_buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                break
            }
            Err(e) => return Err(e.into()),
        };
        buffer.extend_from_slice(&temp_buf[..n]);

        if header_end.is_none() {
            if let Some(pos) = find_subsequence(&buffer, b"\r\n\r\n") {
                header_end = Some(pos + 4);
                let header_str = String::from_utf8_lossy(&buffer[..pos]);
                for line in header_str.lines() {
                    let lower = line.to_ascii_lowercase();
                    if lower.starts_with("content-length:") {
                        if let Ok(len) = lower[15..].trim().parse::<usize>() {
                            content_length = Some(len);
                        }
                    }
                }
            }
        }

        if let Some(h_end) = header_end {
            if let Some(c_len) = content_length {
                if buffer.len() >= h_end + c_len {
                    break;
                }
            } else {
                // No body expected (e.g. GET)
                break;
            }
        }
        if buffer.len() > 65536 {
            break;
        }
    }

    if buffer.is_empty() {
        return Ok(false);
    }

    let request_str = String::from_utf8_lossy(&buffer);
    let mut lines = request_str.lines();
    let req_line = match lines.next() {
        Some(l) => l,
        None => return Ok(false),
    };

    let parts: Vec<&str> = req_line.split_whitespace().collect();
    if parts.len() < 2 {
        send_response(stream, 400, "Bad Request", "text/plain", "Bad Request")?;
        return Ok(false);
    }

    let method = parts[0];
    let path = parts[1];

    match (method, path) {
        ("GET", "/health") => {
            let resp = HealthResponse {
                status: "ok".to_string(),
                model: model_path.to_string_lossy().to_string(),
                idle_remaining_s,
            };
            let body = serde_json::to_string(&resp)?;
            send_response(stream, 200, "OK", "application/json", &body)?;
            Ok(false)
        }
        ("POST", "/shutdown") => {
            send_response(
                stream,
                200,
                "OK",
                "application/json",
                r#"{"status":"shutting_down"}"#,
            )?;
            Ok(true)
        }
        ("POST", "/synth") => {
            // Find body in HTTP request
            let body_start = request_str.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
            let body_str = &request_str[body_start..];
            let req: SynthRequest = match serde_json::from_str(body_str) {
                Ok(r) => r,
                Err(e) => {
                    let err_msg = format!(r#"{{"error":"invalid_json: {}"}}"#, e);
                    send_response(stream, 400, "Bad Request", "application/json", &err_msg)?;
                    return Ok(false);
                }
            };

            let param_names = req.params.join(" ");
            let has_directive = req
                .directive
                .as_ref()
                .map(|d| !d.trim().is_empty())
                .unwrap_or(false);
            let pir_prompt = if let Some(d) = &req.directive {
                if !d.trim().is_empty() {
                    format!("Prompt: {}\n<pir>\n", d.trim())
                } else {
                    format!("f {} ({}) [\n", req.symbol, param_names)
                }
            } else {
                format!("f {} ({}) [\n", req.symbol, param_names)
            };

            let channel = SparseChannel::Pir;
            let constraint = Box::new(engine.build_channel_constraint(channel, Some(&pir_prompt)));
            let mut sampler = Sampler::new(0.0, 1.0, 1, 1.0, constraint);
            let mut generated_buf = String::new();

            let metrics = engine.generate(&pir_prompt, &mut sampler, 192, |tok| {
                generated_buf.push_str(tok);
            })?;

            let full_pir = if has_directive {
                generated_buf.clone()
            } else {
                format!("{}{}", pir_prompt, generated_buf)
            };
            let synthesized_psudo =
                if let Some(psudo_out) = pir_decompiler::decompile_pir_to_psudo(&full_pir) {
                    if has_directive {
                        rebind_psudo_signature(&psudo_out, &req.symbol, &req.params)
                    } else {
                        psudo_out
                    }
                } else {
                    format!("fn {}({})\n    return 0", req.symbol, req.params.join(", "))
                };

            let resp = SynthResponse {
                symbol: req.symbol,
                psudo: synthesized_psudo,
                decode_tps: metrics.decode_tps as f32,
            };
            let body = serde_json::to_string(&resp)?;
            send_response(stream, 200, "OK", "application/json", &body)?;
            Ok(false)
        }
        _ => {
            send_response(stream, 404, "Not Found", "text/plain", "Not Found")?;
            Ok(false)
        }
    }
}

fn rebind_psudo_signature(psudo_code: &str, target_sym: &str, target_params: &[String]) -> String {
    let mut lines = psudo_code.lines();
    let Some(first) = lines.next() else {
        return psudo_code.to_string();
    };
    let rest: Vec<&str> = lines.collect();
    let mut gen_params: Vec<String> = Vec::new();
    if let (Some(lpar), Some(rpar)) = (first.find('('), first.rfind(')')) {
        if lpar < rpar {
            gen_params = first[lpar + 1..rpar]
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }
    let header = format!("fn {}({})", target_sym, target_params.join(", "));
    let mut body = rest.join("\n");
    if gen_params.len() == target_params.len() {
        for (old_p, new_p) in gen_params.iter().zip(target_params.iter()) {
            if old_p != new_p && !old_p.is_empty() {
                body = replace_ident(&body, old_p, new_p);
            }
        }
    } else {
        for (idx, old_p) in gen_params.iter().enumerate() {
            if !old_p.is_empty() && !target_params.contains(old_p) {
                if let Some(new_p) = target_params.get(idx).or_else(|| target_params.last()) {
                    body = replace_ident(&body, old_p, new_p);
                }
            }
        }
    }

    let unbound = collect_unbound_idents(&body, target_params);
    for (idx, ident) in unbound.iter().enumerate() {
        let replacement = target_params
            .get(idx)
            .or_else(|| target_params.last())
            .map(String::as_str)
            .unwrap_or("0");
        body = replace_ident(&body, ident, replacement);
    }

    if body.is_empty() {
        header
    } else {
        format!("{}\n{}", header, body)
    }
}

fn is_psudo_reserved_or_builtin(ident: &str) -> bool {
    matches!(
        ident,
        "let"
            | "if"
            | "else"
            | "elif"
            | "while"
            | "for"
            | "in"
            | "return"
            | "fn"
            | "true"
            | "false"
            | "null"
            | "and"
            | "or"
            | "not"
            | "print"
            | "len"
            | "str"
            | "num"
            | "bool"
            | "abs"
            | "min"
            | "max"
            | "pow"
            | "sqrt"
            | "floor"
            | "ceil"
            | "round"
            | "load_json"
            | "readFile"
            | "writeFile"
            | "persistent_kv"
            | "get"
            | "set"
    )
}

fn collect_unbound_idents(body: &str, target_params: &[String]) -> Vec<String> {
    let mut locals: Vec<String> = target_params.to_vec();
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(after_let) = trimmed.strip_prefix("let ") {
            let lhs = after_let
                .split('=')
                .next()
                .unwrap_or("")
                .trim();
            if !lhs.is_empty() && lhs.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                locals.push(lhs.to_string());
            }
        }
    }

    let mut unbound = Vec::new();
    let mut in_str = false;
    let mut escaped = false;
    let mut cur = String::new();
    let chars: Vec<char> = body.chars().collect();
    for (idx, &ch) in chars.iter().enumerate() {
        if in_str {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_str = false;
            }
            continue;
        }
        if ch == '"' {
            cur.clear();
            in_str = true;
            continue;
        }
        if ch.is_ascii_alphanumeric() || ch == '_' {
            cur.push(ch);
        } else {
            if !cur.is_empty() {
                let is_call = ch == '(';
                let starts_ident = cur
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
                if starts_ident
                    && !is_call
                    && !is_psudo_reserved_or_builtin(&cur)
                    && !locals.contains(&cur)
                    && !unbound.contains(&cur)
                {
                    unbound.push(cur.clone());
                }
                cur.clear();
            }
        }
        let _ = idx;
    }
    if !cur.is_empty() {
        let starts_ident = cur
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
        if starts_ident
            && !is_psudo_reserved_or_builtin(&cur)
            && !locals.contains(&cur)
            && !unbound.contains(&cur)
        {
            unbound.push(cur);
        }
    }
    unbound
}

fn replace_ident(src: &str, old: &str, new: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut cur = String::new();
    let mut in_str = false;
    let mut escaped = false;
    for ch in src.chars() {
        if in_str {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_str = false;
            }
            continue;
        }
        if ch == '"' {
            if !cur.is_empty() {
                out.push_str(if cur == old { new } else { &cur });
                cur.clear();
            }
            in_str = true;
            out.push(ch);
            continue;
        }
        if ch.is_ascii_alphanumeric() || ch == '_' {
            cur.push(ch);
        } else {
            if !cur.is_empty() {
                out.push_str(if cur == old { new } else { &cur });
                cur.clear();
            }
            out.push(ch);
        }
    }
    if !cur.is_empty() {
        out.push_str(if cur == old { new } else { &cur });
    }
    out
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn send_response(
    stream: &mut TcpStream,
    status_code: u16,
    status_text: &str,
    content_type: &str,
    body: &str,
) -> Result<(), std::io::Error> {
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status_code,
        status_text,
        content_type,
        body.len(),
        body
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    let _ = stream.shutdown(std::net::Shutdown::Both);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_json() {
        let resp = HealthResponse {
            status: "ok".to_string(),
            model: "pseudo-33m-tq2_0.gguf".to_string(),
            idle_remaining_s: 600,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains(r#""status":"ok""#));
        assert!(json.contains(r#""idle_remaining_s":600"#));
    }

    #[test]
    fn test_synth_request_deserialization() {
        let json = r#"{"symbol":"fib","params":["n"],"directive":"compute nth Fibonacci"}"#;
        let req: SynthRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.symbol, "fib");
        assert_eq!(req.params, vec!["n"]);
        assert_eq!(req.directive.as_deref(), Some("compute nth Fibonacci"));
    }

    #[test]
    fn test_synth_response_json() {
        let resp = SynthResponse {
            symbol: "fib".to_string(),
            psudo: "fn fib(n)\n    return n".to_string(),
            decode_tps: 42.0,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains(r#""symbol":"fib""#));
        assert!(json.contains("fn fib(n)"));
    }

    #[test]
    fn test_rebind_same_arity() {
        let gen = "fn clamp(val, min_val, max_val)\n    if val < min_val\n        return min_val\n    return val";
        let out = rebind_psudo_signature(
            gen,
            "clamp_status",
            &["status".into(), "lo".into(), "hi".into()],
        );
        assert!(out.starts_with("fn clamp_status(status, lo, hi)"));
        assert!(out.contains("if status < lo"));
        assert!(out.contains("return lo"));
    }

    #[test]
    fn test_rebind_arity_mismatch_and_unbound_fallback() {
        let gen = "fn in_range(val, low, high)\n    if val >= low\n        return val <= high\n    return false";
        let out = rebind_psudo_signature(gen, "is_server_error", &["status".into()]);
        assert!(out.starts_with("fn is_server_error(status)"));
        assert!(!out.contains("val"));
        assert!(!out.contains("low"));
        assert!(!out.contains("high"));
        assert!(out.contains("status"));
    }

    #[test]
    fn test_rebind_hallucinated_body_vars() {
        let gen = "fn clamp_status(status, lo, hi)\n    if celsius >= 40\n        return code + 1\n    return st";
        let out = rebind_psudo_signature(
            gen,
            "clamp_status",
            &["status".into(), "lo".into(), "hi".into()],
        );
        assert!(!out.contains("celsius"));
        assert!(!out.contains("code"));
        assert!(!out.contains("return st\n"));
        assert!(out.contains("status >= 40"));
    }
}
