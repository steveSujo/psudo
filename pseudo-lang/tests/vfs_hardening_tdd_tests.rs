use psudo::vfs::WorkspaceCapsule;
use std::fs;
use std::process::Command;

#[test]
fn test_vfs_hardening_and_workspace_artifacts_tdd() {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let test_dir = std::env::temp_dir().join(format!("psudo_tdd_hardening_{}", millis));
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();

    let psudo_bin = env!("CARGO_BIN_EXE_psudo");

    // 1. Test psudo init produces clean capsule without docs, without templates
    let status = Command::new(psudo_bin)
        .arg("init")
        .arg(&test_dir)
        .status()
        .expect("Failed to run psudo init");
    assert!(status.success());

    let bin_path = test_dir.join("psudo.bin");
    assert!(bin_path.exists());

    // 2. Load capsule and verify: NO documentation files inside VFS! Only code/config.
    let capsule =
        WorkspaceCapsule::load_from_file(&bin_path).expect("Failed to load signed capsule");
    assert!(
        capsule.virtual_files.contains_key("src/main.psudo"),
        "Must contain entrypoint src/main.psudo"
    );
    assert!(
        capsule.virtual_files.contains_key("psudo.toml"),
        "Must contain psudo.toml"
    );
    assert!(
        !capsule.virtual_files.contains_key("AGENTS.md"),
        "NO docs allowed inside VFS"
    );
    assert!(
        !capsule.virtual_files.contains_key("USER_GUIDE.md"),
        "NO docs allowed inside VFS"
    );
    assert!(
        !capsule.virtual_files.contains_key("CLAUDE.md"),
        "NO docs allowed inside VFS"
    );

    // 3. Test tamper-resistance and signature verification
    let mut bin_bytes = fs::read(&bin_path).unwrap();
    // Tamper with one payload byte
    let last_idx = bin_bytes.len() - 1;
    bin_bytes[last_idx] ^= 0xFF;
    let tampered_path = test_dir.join("tampered.bin");
    fs::write(&tampered_path, &bin_bytes).unwrap();
    let tamper_load = WorkspaceCapsule::load_from_file(&tampered_path);
    assert!(
        tamper_load.is_err(),
        "Tampered capsule must fail signature verification!"
    );

    // 4. Test code reveal gating (--reveal-code <KEY>)
    // Default show on .psudo without key should NOT reveal full code when --all is requested
    let show_blocked = Command::new(psudo_bin)
        .arg("vfs")
        .arg("show")
        .arg(&bin_path)
        .arg("src/main.psudo")
        .arg("--all")
        .output()
        .expect("Failed to run psudo vfs show");
    assert!(show_blocked.status.success());
    let blocked_out = String::from_utf8_lossy(&show_blocked.stdout);
    assert!(blocked_out.contains("Code reveal locked") || !blocked_out.contains("return 0"));

    // With magic key, it reveals code
    let show_revealed = Command::new(psudo_bin)
        .arg("vfs")
        .arg("show")
        .arg(&bin_path)
        .arg("src/main.psudo")
        .arg("--all")
        .arg("--reveal-code")
        .arg("the slow pink elephant")
        .output()
        .expect("Failed to run psudo vfs show with key");
    assert!(show_revealed.status.success());
    let revealed_out = String::from_utf8_lossy(&show_revealed.stdout);
    assert!(revealed_out.contains("fn main"));

    // 5. Test build outputs to local workspace (.pseudo/build and .pseudo/bin) NOT in temp!
    let build_output = Command::new(psudo_bin)
        .arg("build")
        .arg(&bin_path)
        .output()
        .expect("Failed to run psudo build");
    assert!(build_output.status.success(), "Build must succeed");

    let expected_local_bin = test_dir.join(".pseudo").join("bin");
    let expected_local_build = test_dir.join(".pseudo").join("build");
    assert!(
        expected_local_build.exists(),
        "Build artifact directory must exist in workspace at .pseudo/build"
    );
    assert!(
        expected_local_bin.exists(),
        "Binary directory must exist in workspace at .pseudo/bin"
    );

    let entries: Vec<_> = fs::read_dir(&expected_local_bin).unwrap().collect();
    assert!(
        !entries.is_empty(),
        "Executable must be placed in workspace .pseudo/bin/"
    );

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);
}
