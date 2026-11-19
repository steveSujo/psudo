use std::path::Path;
use std::process::Command;

#[test]
fn test_user_end_mcp_rig() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let rig_path = Path::new(manifest_dir)
        .parent()
        .expect("parent dir")
        .join("playground")
        .join("mcp-test")
        .join("run_mcp_rig.py");

    if !rig_path.exists() {
        return;
    }

    let status = Command::new("python")
        .arg(&rig_path)
        .status()
        .expect("Failed to execute MCP test rig via python");

    assert!(
        status.success(),
        "MCP test rig (run_mcp_rig.py) failed with status: {:?}",
        status
    );
}
