use psudo::vfs::WorkspaceCapsule;

#[test]
fn test_vfs_stub_upsert_and_extraction() {
    let mut capsule = WorkspaceCapsule::new_empty("test-app");

    // Upsert a stub
    let params = vec!["subtotal".to_string(), "rate".to_string()];
    capsule
        .upsert_stub(
            "src/main.psudo",
            "calculate_tax",
            &params,
            "calculate tax as (subtotal * rate) / 100",
        )
        .unwrap();

    // Extract stubs
    let stubs = capsule.extract_stubs("src/main.psudo").unwrap();
    assert_eq!(stubs.len(), 1);
    assert_eq!(stubs[0].name, "calculate_tax");
    assert_eq!(stubs[0].params, vec!["subtotal", "rate"]);
    assert_eq!(
        stubs[0].directive,
        "calculate tax as (subtotal * rate) / 100"
    );

    // Format pIR
    let pir_output = capsule.format_stubs_pir(&stubs);
    assert!(pir_output.contains(
        "f calculate_tax (subtotal rate) [ @stub \"calculate tax as (subtotal * rate) / 100\" ]"
    ));

    // Format human
    let human_output = capsule.format_stubs_human(&stubs);
    assert!(human_output.contains("fn calculate_tax(subtotal, rate)"));
    assert!(human_output.contains("@stub(\"calculate tax as (subtotal * rate) / 100\")"));
}

#[test]
fn test_vfs_occlusion_zero_stubs() {
    let mut capsule = WorkspaceCapsule::new_empty("test-app");
    // Fully synthesized code (no stubs)
    capsule
        .set_file("src/main.psudo", "fn main()\n    return 0\n".to_string())
        .unwrap();

    let stubs = capsule.extract_stubs("src/main.psudo").unwrap();
    assert!(stubs.is_empty());
}

#[test]
fn test_vfs_stub_param_validation() {
    let psudo_bin = env!("CARGO_BIN_EXE_psudo");
    let output = std::process::Command::new(psudo_bin)
        .args([
            "vfs",
            "stub",
            ".",
            "fib",
            "invalid param",
            "--directive",
            "compute",
        ])
        .output()
        .expect("Failed to run psudo vfs stub");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Invalid parameter name"));
}

#[test]
fn test_vfs_stub_missing_fn_validation() {
    let psudo_bin = env!("CARGO_BIN_EXE_psudo");
    let output = std::process::Command::new(psudo_bin)
        .args(["vfs", "stub", "."])
        .output()
        .expect("Failed to run psudo vfs stub");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Function name is required"));
}
