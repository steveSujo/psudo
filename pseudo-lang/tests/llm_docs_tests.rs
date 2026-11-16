use std::process::Command;

#[test]
fn test_llm_cli_docs_content() {
    let docs = psudo::user_docs::llm_cli_docs();
    assert!(!docs.is_empty(), "LLM CLI docs must not be empty");

    // Verify user-facing domain terms
    assert!(
        docs.contains("capsule"),
        "Must contain domain term 'capsule'"
    );
    assert!(docs.contains("stub"), "Must contain domain term 'stub'");
    assert!(docs.contains("GGUF"), "Must contain domain term 'GGUF'");
    assert!(
        docs.contains("Iterating on Stubs"),
        "Must contain stub iteration guidance"
    );
    assert!(docs.contains("Anti-Patterns"), "Must contain anti-patterns");

    // Verify actionable subcommands
    assert!(docs.contains("init"), "Must contain subcommand 'init'");
    assert!(docs.contains("vfs"), "Must contain subcommand 'vfs'");
    assert!(docs.contains("check"), "Must contain subcommand 'check'");
    assert!(docs.contains("synth"), "Must contain subcommand 'synth'");
    assert!(docs.contains("build"), "Must contain subcommand 'build'");
    assert!(docs.contains("test"), "Must contain subcommand 'test'");
    assert!(
        docs.contains("benchmark"),
        "Must contain subcommand 'benchmark'"
    );
    assert!(
        docs.contains("contract"),
        "Must contain subcommand 'contract'"
    );
    assert!(docs.contains("run"), "Must contain subcommand 'run'");
}

#[test]
fn test_cli_docs_flag_and_subcommand() {
    let bin = env!("CARGO_BIN_EXE_psudo");

    // Test 'docs' subcommand
    let output_sub = Command::new(bin)
        .arg("docs")
        .output()
        .expect("Failed to execute psudo docs");
    assert!(output_sub.status.success(), "psudo docs failed");
    let stdout_sub = String::from_utf8_lossy(&output_sub.stdout);
    assert!(
        stdout_sub.contains("capsule"),
        "Subcommand output missing terms"
    );

    // Test '--llm-docs' flag
    let output_flag = Command::new(bin)
        .arg("--llm-docs")
        .output()
        .expect("Failed to execute psudo --llm-docs");
    assert!(output_flag.status.success(), "psudo --llm-docs failed");
    let stdout_flag = String::from_utf8_lossy(&output_flag.stdout);
    assert!(stdout_flag.contains("capsule"), "Flag output missing terms");
}

#[test]
fn test_main_help_workflow_guidance() {
    let bin = env!("CARGO_BIN_EXE_psudo");
    let output = Command::new(bin)
        .arg("--help")
        .output()
        .expect("Failed to execute psudo --help");
    assert!(output.status.success(), "psudo --help failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("CANONICAL AI / DEVELOPER WORKFLOW"),
        "Missing workflow header"
    );
    assert!(stdout.contains("GGUF"), "Missing GGUF reference");
    assert!(
        stdout.contains("psudo synth"),
        "Missing synth command reference"
    );
    assert!(
        stdout.contains("ITERATING / REFINING EXISTING STUBS"),
        "Missing stub iteration guidance"
    );
    assert!(
        stdout.contains("DO NOT read, parse, or edit"),
        "Missing binary tampering warning"
    );
    assert!(
        stdout.contains("psudo contract"),
        "Missing contract command reference"
    );
}

#[test]
fn test_subcommand_help_interceptors() {
    let bin = env!("CARGO_BIN_EXE_psudo");
    let subcommands = vec!["synth", "vfs", "check", "init", "contract", "build"];
    for sub in subcommands {
        let output = Command::new(bin)
            .arg(sub)
            .arg("--help")
            .output()
            .unwrap_or_else(|_| panic!("Failed to execute psudo {} --help", sub));
        assert!(output.status.success(), "psudo {} --help failed", sub);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(!stdout.is_empty(), "Subcommand {} --help output empty", sub);
    }
}
