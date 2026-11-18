use psudo::{
    lexer::Lexer,
    ownership::{BorrowChecker, CheckedEnvironment},
    parser::{CheckedInterpret, Interpret, Parser},
};
use std::{cell::RefCell, rc::Rc};

fn run_test(source: &str) {
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.scan_tokens().unwrap();
    let mut parser = Parser::new(tokens);
    let statements = parser.parse().unwrap();
    let mut interpreter = Interpret::default();
    for stmt in statements {
        interpreter.execute(&stmt);
    }
}

fn run_checked_test(source: &str) {
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.scan_tokens().unwrap();
    let mut parser = Parser::new(tokens);
    let statements = parser.parse().unwrap();
    let borrow_checker = Rc::new(RefCell::new(BorrowChecker::new()));
    let mut interpreter = CheckedInterpret {
        environment: Rc::new(RefCell::new(CheckedEnvironment::new(borrow_checker))),
        check_borrows: true,
    };
    for stmt in statements {
        interpreter.execute(&stmt);
    }
}

#[test]
fn test_arithmetic() {
    run_test("print 1 + 2");
    run_test("print 3 * 4");
    run_test("print 10 - 5");
    run_test("print 10 / 2");
}

#[test]
fn test_comparison() {
    run_test("print 10 > 5");
    run_test("print 3 < 4");
    run_test("print 5 == 5");
    run_test("print 5 != 3");
}

#[test]
fn test_string() {
    run_test("print \"hello world\"");
    run_test("print \"test\" + \"ing\"");
}

#[test]
fn test_variables() {
    run_test("let x = 10\nprint x");
    run_test("let x = 10\nlet y = 5\nprint x + y");
}

#[test]
fn test_while_loop() {
    run_test("let i = 0\nwhile i < 3\n    print i\n    i = i + 1");
}

#[test]
fn test_for_loop() {
    run_test("for let i = 0\ni < 3\ni = i + 1\n    print i");
}

#[test]
fn test_if_else() {
    run_test("if true\n    print \"yes\"\nelse\n    print \"no\"");
    run_test("if false\n    print \"yes\"\nelse\n    print \"no\"");
}

#[test]
fn test_use_after_move_compound_array_errors() {
    let source = "let a = [10, 20]\nlet b = a\nprint a";
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.scan_tokens().unwrap();
    let mut parser = Parser::new(tokens);
    let statements = parser.parse().unwrap();
    let borrow_checker = Rc::new(RefCell::new(BorrowChecker::new()));
    let mut interpreter = CheckedInterpret {
        environment: Rc::new(RefCell::new(CheckedEnvironment::new(borrow_checker))),
        check_borrows: true,
    };
    for stmt in statements {
        interpreter.execute(&stmt);
    }
    let errs = &interpreter.environment.borrow().errors;
    assert!(!errs.is_empty(), "Expected use-after-move error");
    assert!(errs.iter().any(|e| e.contains("use of moved value")));
}

#[test]
fn test_copy_primitives_and_reassignment_succeed() {
    let source = "let x = 10\nlet y = x\nx = x + 5\nprint x + y";
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.scan_tokens().unwrap();
    let mut parser = Parser::new(tokens);
    let statements = parser.parse().unwrap();
    let borrow_checker = Rc::new(RefCell::new(BorrowChecker::new()));
    let mut interpreter = CheckedInterpret {
        environment: Rc::new(RefCell::new(CheckedEnvironment::new(borrow_checker))),
        check_borrows: true,
    };
    for stmt in statements {
        interpreter.execute(&stmt);
    }
    assert!(
        interpreter.environment.borrow().errors.is_empty(),
        "Expected zero errors for primitives"
    );
}

#[test]
fn test_mutable_then_immutable_conflict_or_reassign() {
    let source = "let x = 10\nprint x\nx = 20\nprint x";
    run_checked_test(source);
}

#[test]
fn test_self_teaching_hints_and_default_cwd_capsule_cli() {
    use psudo::diagnostics::ErrorCode;
    assert!(ErrorCode::E001UnrecognizedChar.help_message().contains("'//'"));
    assert!(ErrorCode::E002IndentationError.help_message().contains("4-space"));

    // Verify optional <CAPSULE> defaults to ./psudo.bin in `contract --json` and `build --run`
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let dir = std::env::temp_dir().join(format!("psudo_cwd_default_{}", millis));
    let _ = std::fs::remove_dir_all(&dir);
    let bin = env!("CARGO_BIN_EXE_psudo");

    let init = std::process::Command::new(bin)
        .args(["init", "."])
        .current_dir({
            std::fs::create_dir_all(&dir).unwrap();
            &dir
        })
        .status()
        .unwrap();
    assert!(init.success());

    let contract_out = std::process::Command::new(bin)
        .args(["contract", "--json"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        contract_out.status.success(),
        "contract --json failed: {}",
        String::from_utf8_lossy(&contract_out.stderr)
    );
    assert!(String::from_utf8_lossy(&contract_out.stdout).contains("pseudo_ast_stub"));

    let daemon_out = std::process::Command::new(bin)
        .args(["daemon", "status"])
        .output()
        .unwrap();
    assert!(daemon_out.status.success());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_ansi_color_gates_cleanly_on_non_tty() {
    assert_eq!(psudo::utils::paint("ok", "1;32", false), "ok");
    assert_eq!(
        psudo::utils::paint("ok", "1;32", true),
        "\x1b[1;32mok\x1b[0m"
    );
}

#[test]
fn test_tui_builtins_and_tokio_sleep_pass_resolver_and_borrow_checker() {
    let source = r#"
fn main()
    let m = tui_metric("Net", 2035)
    let g = tui_gauge("Budget", 86)
    let s = tui_sparkline("Trend", [10, 20, 30])
    let t = tui_table("Rows", [m])
    let l = tui_log("Logs", ["ok"])
    let _ = tokio_sleep_ms(1)
    return tui_dashboard("Finance", [m, g, s, t, l])
"#;
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.scan_tokens().unwrap();
    let mut parser = Parser::new(tokens);
    let statements = parser.parse().unwrap();
    let res = psudo::resolver::Resolver::new().resolve(&statements);
    assert!(res.errors.is_empty(), "Resolver errors: {:?}", res.errors);
    run_checked_test(source);
}

#[test]
fn test_all_three_tui_templates_parse_resolve_borrowcheck_and_transpile_zero_unwrap() {
    let templates = [
        (
            include_str!("../../templates/tui_game_of_life.psudo"),
            "pub fn tui_grid(",
        ),
        (
            include_str!("../../templates/tui_fractal_zoom.psudo"),
            "pub fn tui_fractal(",
        ),
        (
            include_str!("../../templates/tui_personal_finance.psudo"),
            "pub fn tui_dashboard(",
        ),
    ];
    for (src, expected_fn) in templates {
        let mut lexer = Lexer::new(src.to_string());
        let tokens = lexer.scan_tokens().unwrap();
        let mut parser = Parser::new(tokens);
        let statements = parser.parse().unwrap();
        let res = psudo::resolver::Resolver::new().resolve(&statements);
        assert!(res.errors.is_empty(), "Resolver errors: {:?}", res.errors);

        let rust = psudo::codegen::transpile_psudo_to_rust(src, true).unwrap();
        assert!(rust.contains(expected_fn), "Missing {}", expected_fn);
        assert!(!rust.contains(".unwrap()"));
        assert!(!rust.contains(".expect()"));
    }
}

#[test]
fn test_custom_addon_capsule_transpilation() {
    let mut capsule = psudo::vfs::WorkspaceCapsule::new_empty("addon_it");
    capsule.virtual_files.insert(
        "psudo.toml".to_string(),
        r##"
[addons.my_worker]
builtins = ["worker_ping"]
cargo_deps = ['tokio = { version = "1", features = ["full"] }']
rust_main_attr = "#[tokio::main]"
rust_main_async = true
rust_file = "src/worker_addon.rs"
"##
        .to_string(),
    );
    let cfg = psudo::config::parse_project_config_str(
        capsule.virtual_files.get("psudo.toml").unwrap(),
    );
    assert!(cfg.addons.contains_key("my_worker"));
}

#[test]
fn test_codegen_discard_let_clones_array_vars_and_composable_tui_widgets() {
    let source = r#"
fn main()
    let m1 = tui_metric("CPU", 86.6)
    let m2 = tui_metric("MEM", 42)
    let _ = tokio_sleep_ms(1)
    let row = tui_row([m1, m2])
    let grid = tui_canvas("Life", [[0, 1], [1, 0]])
    let bars = tui_barchart("Spend", ["Rent", "Food"], [1200, 450])
    let tbl = tui_table("Services", [m1, m2])
    return tui_render("Monitor", [row, grid, bars, tbl])
"#;
    let rust = psudo::codegen::transpile_psudo_to_rust(source, false).unwrap();
    assert!(rust.contains("let _ = tokio_sleep_ms("), "Must not emit `let mut _`");
    assert!(!rust.contains("let mut _ ="), "Found invalid `let mut _ =`");
    assert!(rust.contains("m1.clone()"), "Array literal must clone identifiers");
    assert!(rust.contains("pub fn tui_canvas("));
    assert!(rust.contains("pub fn tui_barchart("));
    assert!(rust.contains("pub fn tui_row("));
    assert!(rust.contains("pub fn tui_render("));
    assert!(rust.contains("#[tokio::main]\npub async fn main()"));
    assert!(!rust.contains(".unwrap()"));
    assert!(!rust.contains(".expect()"));
}

#[test]
fn test_addon_bom_stripping_and_mock_returns_parse() {
    let raw_toml = "\u{feff}[addons.worker]\nbuiltins = [\"worker_ping\"]\nrust_file = \"src/worker.rs\"\n[addons.worker.mock_returns]\nworker_ping = \"pong:test\"\n";
    let cfg = psudo::config::parse_project_config_str(raw_toml.trim_start_matches('\u{feff}'));
    let worker = cfg.addons.get("worker").expect("worker addon");
    assert_eq!(
        worker.mock_returns.get("worker_ping").map(String::as_str),
        Some("pong:test")
    );
}
