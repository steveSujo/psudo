use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

#[test]
fn test_mcp_stdio_jsonrpc_workflow() {
    let mut child = Command::new("cargo")
        .arg("run")
        .arg("--manifest-path")
        .arg("../pseudo-lang/Cargo.toml")
        .arg("--")
        .arg("mcp")
        .arg("--in-process")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cargo run mcp");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut reader = BufReader::new(stdout);

    // 1. Initialize
    let init_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "test-client", "version": "1.0" }
        }
    });
    writeln!(stdin, "{}", init_req.to_string()).unwrap();
    stdin.flush().unwrap();

    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let resp: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 1);

    // 2. Resources Read (psudo://syntax)
    let res_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "resources/read",
        "params": { "uri": "psudo://syntax" }
    });
    writeln!(stdin, "{}", res_req.to_string()).unwrap();
    stdin.flush().unwrap();

    line.clear();
    reader.read_line(&mut line).unwrap();
    let res_resp: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(res_resp["id"], 2);
    assert!(res_resp["result"]["contents"][0]["text"].as_str().unwrap().contains("psudo syntax"));

    // 3. Tools List
    let tools_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/list",
        "params": {}
    });
    writeln!(stdin, "{}", tools_req.to_string()).unwrap();
    stdin.flush().unwrap();

    line.clear();
    reader.read_line(&mut line).unwrap();
    let tools_resp: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(tools_resp["id"], 3);
    assert!(tools_resp["result"]["tools"].as_array().unwrap().len() >= 8);

    let _ = child.kill();
}
