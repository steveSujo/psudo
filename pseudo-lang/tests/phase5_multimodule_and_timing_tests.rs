use psudo::lexer::Lexer;
use psudo::parser::{Interpret, Parser};
use psudo::vfs::WorkspaceCapsule;

#[test]
fn test_timing_builtins_monotonic() {
    let source = "let t1 = time_micros()\nlet t2 = time_micros()\nassert(t2 >= t1, \"time should be monotonic\")\nlet n1 = time_nanos()\nassert(n1 >= 0, \"nanos should be non-negative\")\nlet s1 = time_now_secs()\nassert(s1 > 0, \"epoch seconds should be positive\")\n";
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.scan_tokens().unwrap();
    let mut parser = Parser::new(tokens);
    let stmts = parser.parse().unwrap();
    let mut interpreter = Interpret::default();
    for stmt in stmts {
        interpreter.execute(&stmt);
    }
}

#[test]
fn test_sleep_ms_delay() {
    let source = "let start = time_micros()\nlet ok = sleep_ms(10)\nlet end = time_micros()\nassert(ok, \"sleep should return true\")\nassert(end >= start, \"elapsed should be valid\")\n";
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.scan_tokens().unwrap();
    let mut parser = Parser::new(tokens);
    let stmts = parser.parse().unwrap();
    let mut interpreter = Interpret::default();
    for stmt in stmts {
        interpreter.execute(&stmt);
    }
}

#[test]
fn test_benchmark_helper() {
    let source = "let metrics = benchmark(10, fn()\n    let x = 1 + 1\n    return x\n)\nassert(metrics[\"iterations\"] == 10, \"iterations should match\")\n";
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.scan_tokens().unwrap();
    let mut parser = Parser::new(tokens);
    let stmts = parser.parse().unwrap();
    let mut interpreter = Interpret::default();
    for stmt in stmts {
        interpreter.execute(&stmt);
    }
}

#[test]
fn test_multimodule_capsule_structure() {
    let mut capsule = WorkspaceCapsule::new_empty("test_project");
    capsule
        .virtual_files
        .insert("src/main.psudo".into(), "fn main()\n    return 0".into());
    capsule.virtual_files.insert(
        "src/models.psudo".into(),
        "fn get_model()\n    return \"tq2_0\"".into(),
    );

    let mut total_stubs = 0;
    for (vpath, source) in &capsule.virtual_files {
        if vpath.ends_with(".psudo") {
            let digest = psudo::context::analyze_psudo_source(source, vpath, None).unwrap();
            total_stubs += digest.symbols.iter().filter(|s| s.is_stub).count();
        }
    }
    assert_eq!(
        total_stubs, 0,
        "Clean multi-module capsule should have zero stubs"
    );
}
