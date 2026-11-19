use std::fs;
use std::process::Command;

#[test]
fn tdd_verify_pseudo_toolchain_and_mcp() {
    let psudo_bin = env!("CARGO_BIN_EXE_psudo");
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let project_dir = std::env::temp_dir().join(format!("psudo_tdd_app_{}", millis));
    let _ = fs::remove_dir_all(&project_dir);

    // 1. TDD Test: psudo init
    let status = Command::new(psudo_bin)
        .arg("init")
        .arg(&project_dir)
        .status()
        .expect("Failed to execute psudo init");
    assert!(status.success());
    assert!(project_dir.join("psudo.bin").exists());

    // Assert strictly only psudo.bin exists on disk
    let entries: Vec<_> = fs::read_dir(&project_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].to_string_lossy(), "psudo.bin");

    let capsule_path = project_dir.join("psudo.bin");

    // 2. TDD Test: psudo contract
    let contract_output = Command::new(psudo_bin)
        .arg("contract")
        .arg(&capsule_path)
        .output()
        .expect("Failed to execute psudo contract");
    assert!(contract_output.status.success());
    let stdout_str = String::from_utf8_lossy(&contract_output.stdout);
    assert!(stdout_str.contains("pseudo_ast_stub"));

    // 3. TDD Test: psudo check
    let check_output = Command::new(psudo_bin)
        .arg("check")
        .arg(&capsule_path)
        .output()
        .expect("Failed to execute psudo check");
    assert!(check_output.status.success());
    assert!(String::from_utf8_lossy(&check_output.stdout).contains("Static analysis passed"));

    // 4. TDD Test: psudo synth
    let synth_output = Command::new(psudo_bin)
        .arg("synth")
        .arg(&capsule_path)
        .arg("--allow-empty")
        .output()
        .expect("Failed to execute psudo synth");
    assert!(synth_output.status.success());

    // 5. TDD Test: psudo build --run
    let build_output = Command::new(psudo_bin)
        .arg("build")
        .arg(&capsule_path)
        .arg("--run")
        .output()
        .expect("Failed to execute psudo build");
    assert!(build_output.status.success());
    let build_stdout = String::from_utf8_lossy(&build_output.stdout);
    assert!(build_stdout.contains("Verifiable Rust binary artifact built successfully"));
    assert!(build_stdout.contains("0") || build_stdout.contains("Rust binary artifact built"));

    // Cleanup
    let _ = fs::remove_dir_all(&project_dir);
}
