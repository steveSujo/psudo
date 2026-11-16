use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

#[test]
fn test_single_bin_init_and_vfs_cli() {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let test_dir = std::env::temp_dir().join(format!("psudo_vfs_test_{}", millis));
    let _ = fs::remove_dir_all(&test_dir);

    let psudo_bin = env!("CARGO_BIN_EXE_psudo");

    // 1. Init invoice capsule
    let status = Command::new(psudo_bin)
        .arg("init")
        .arg(&test_dir)
        .arg("--template")
        .arg("invoice")
        .status()
        .expect("Failed to run psudo init");
    assert!(status.success());

    // Assert strictly only psudo.bin exists in test_dir
    let entries: Vec<_> = fs::read_dir(&test_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "Expected strictly 1 file in initialized workspace, found {:?}",
        entries
    );
    assert_eq!(entries[0].to_string_lossy(), "psudo.bin");

    // 2. Test psudo vfs list
    let list_output = Command::new(psudo_bin)
        .arg("vfs")
        .arg("list")
        .arg(&test_dir)
        .output()
        .expect("Failed to run psudo vfs list");
    assert!(list_output.status.success());
    let list_str = String::from_utf8_lossy(&list_output.stdout);
    assert!(list_str.contains("src/main.psudo"));
    assert!(list_str.contains("psudo.toml"));

    // 3. Test psudo vfs show
    let show_output = Command::new(psudo_bin)
        .arg("vfs")
        .arg("show")
        .arg(&test_dir)
        .arg("src/main.psudo")
        .output()
        .expect("Failed to run psudo vfs show");
    assert!(show_output.status.success());
    let show_str = String::from_utf8_lossy(&show_output.stdout);
    assert!(
        show_str.contains("main_task")
            || show_str.contains("0 unresolved stubs")
            || show_str.contains("@stub")
    );

    // 4. Test psudo build --run on capsule
    let build_output = Command::new(psudo_bin)
        .arg("build")
        .arg(test_dir.join("psudo.bin"))
        .arg("--run")
        .output()
        .expect("Failed to run psudo build on capsule");
    assert!(build_output.status.success());
    let build_stdout = String::from_utf8_lossy(&build_output.stdout);
    assert!(build_stdout.contains("Verifiable Rust binary artifact built successfully"));

    let _ = fs::remove_dir_all(&test_dir);
}

fn read_json(reader: &mut BufReader<&mut std::process::ChildStdout>) -> serde_json::Value {
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line).unwrap() == 0 {
            panic!("EOF reached while waiting for JSON-RPC response");
        }
        let trimmed = line.trim();
        if trimmed.starts_with('{') {
            return serde_json::from_str(trimmed)
                .unwrap_or_else(|e| panic!("Failed to parse JSON response '{}': {}", trimmed, e));
        }
    }
}

#[test]
fn test_blackbox_mcp_session_on_single_bin_workspace() {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let test_dir = std::env::temp_dir().join(format!("psudo_mcp_test_{}", millis));
    let _ = fs::remove_dir_all(&test_dir);

    let psudo_bin = env!("CARGO_BIN_EXE_psudo");

    // 1. Init capsule
    let status = Command::new(psudo_bin)
        .arg("init")
        .arg(&test_dir)
        .arg("--template")
        .arg("invoice")
        .status()
        .expect("Failed to run psudo init");
    assert!(status.success());

    let capsule_path = test_dir.join("psudo.bin");
    assert!(capsule_path.exists());

    // 2. Launch psudo mcp --bin capsule_path
    let mut child = Command::new(psudo_bin)
        .arg("mcp")
        .arg("--bin")
        .arg(&capsule_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn psudo mcp");

    let stdin = child.stdin.as_mut().expect("Failed to get stdin");
    let stdout = child.stdout.as_mut().expect("Failed to get stdout");
    let mut reader = BufReader::new(stdout);

    // Send initialize request
    let init_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0"}
        }
    });
    writeln!(stdin, "{}", init_req).unwrap();
    stdin.flush().unwrap();

    let json_resp = read_json(&mut reader);
    assert!(
        json_resp.get("result").is_some(),
        "Expected result in init response: {:?}",
        json_resp
    );

    // Call psudo_context tool
    let ctx_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "psudo_context",
            "arguments": {}
        }
    });
    writeln!(stdin, "{}", ctx_req).unwrap();
    stdin.flush().unwrap();

    let json_resp = read_json(&mut reader);
    assert!(
        json_resp.get("result").is_some(),
        "Expected result in context response: {:?}",
        json_resp
    );

    // Call psudo_vfs_list tool
    let list_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "psudo_vfs_list",
            "arguments": {}
        }
    });
    writeln!(stdin, "{}", list_req).unwrap();
    stdin.flush().unwrap();

    let json_resp = read_json(&mut reader);
    assert!(
        json_resp.get("result").is_some(),
        "Expected result in vfs_list response: {:?}",
        json_resp
    );

    // Call psudo_check tool
    let check_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "psudo_check",
            "arguments": {}
        }
    });
    writeln!(stdin, "{}", check_req).unwrap();
    stdin.flush().unwrap();

    let json_resp = read_json(&mut reader);
    assert!(
        json_resp.get("result").is_some(),
        "Expected result in check response: {:?}",
        json_resp
    );

    // Shutdown / kill child and wait
    let _ = child.kill();
    let _ = child.wait();

    // Verify workspace still contains strictly psudo.bin
    let entries: Vec<_> = fs::read_dir(&test_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].to_string_lossy(), "psudo.bin");

    let _ = fs::remove_dir_all(&test_dir);
}
