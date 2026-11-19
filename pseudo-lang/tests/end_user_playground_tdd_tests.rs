use psudo::vfs::WorkspaceCapsule;
use std::fs;
use std::process::Command;

#[test]
fn tdd_playground_workspaces_and_blackbox_doc_isolation() {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let test_dir = std::env::temp_dir().join(format!("psudo_enduser_test_{}", millis));
    let _ = fs::remove_dir_all(&test_dir);

    let psudo_bin = env!("CARGO_BIN_EXE_psudo");

    // 1. Init blank template
    let blank_dir = test_dir.join("blank_app");
    let status = Command::new(psudo_bin)
        .arg("init")
        .arg(&blank_dir)
        .arg("--template")
        .arg("blank")
        .status()
        .expect("Failed to run psudo init blank");
    assert!(status.success());

    // Assert strictly only psudo.bin exists in blank_dir
    let entries: Vec<_> = fs::read_dir(&blank_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].to_string_lossy(), "psudo.bin");

    let capsule = WorkspaceCapsule::load_from_file(&blank_dir.join("psudo.bin")).unwrap();
    assert!(capsule.virtual_files.contains_key("src/main.psudo"));
    assert!(
        !capsule.virtual_files.contains_key("AGENTS.md"),
        "NO docs allowed inside VFS"
    );
    assert!(
        !capsule.virtual_files.contains_key("USER_GUIDE.md"),
        "NO docs allowed inside VFS"
    );

    // 2. Init another workspace app
    let invoice_dir = test_dir.join("invoice_app");
    let status = Command::new(psudo_bin)
        .arg("init")
        .arg(&invoice_dir)
        .status()
        .expect("Failed to run psudo init invoice");
    assert!(status.success());
    assert!(invoice_dir.join("psudo.bin").exists());

    let status = Command::new(psudo_bin)
        .arg("vfs")
        .arg("stub")
        .arg(invoice_dir.join("psudo.bin"))
        .arg("compute_fee")
        .arg("amount")
        .arg("--directive")
        .arg("return amount * 2")
        .status()
        .expect("Failed to declare @stub");
    assert!(status.success());

    let invoice_capsule = WorkspaceCapsule::load_from_file(&invoice_dir.join("psudo.bin")).unwrap();
    let invoice_source = invoice_capsule.get_file("src/main.psudo").unwrap();
    assert!(invoice_source.contains("@stub"));

    // 3. Test check and synth CLI on invoice app capsule
    let check_status = Command::new(psudo_bin)
        .arg("check")
        .arg(&invoice_dir)
        .status()
        .expect("Failed to run psudo check");
    assert!(check_status.success());

    let synth_status = Command::new(psudo_bin)
        .arg("synth")
        .arg(&invoice_dir)
        .status()
        .expect("Failed to run psudo synth");
    assert!(synth_status.success());

    // 4. Test build --run on synthesized invoice app capsule
    let build_output = Command::new(psudo_bin)
        .arg("build")
        .arg(&invoice_dir)
        .arg("--run")
        .output()
        .expect("Failed to run psudo build");
    if !build_output.status.success() {
        eprintln!("STDOUT: {}", String::from_utf8_lossy(&build_output.stdout));
        eprintln!("STDERR: {}", String::from_utf8_lossy(&build_output.stderr));
    }
    assert!(build_output.status.success());
    let stdout = String::from_utf8_lossy(&build_output.stdout);
    assert!(stdout.contains("0") || stdout.contains("Rust binary artifact built"));

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);
}
