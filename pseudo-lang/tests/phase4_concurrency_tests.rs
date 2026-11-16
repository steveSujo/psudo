use psudo::dag::TaskDAG;
use psudo::lexer::Lexer;
use psudo::parser::{Interpret, Parser};

#[test]
fn test_topological_sort_kahn() {
    let mut dag = TaskDAG::new();
    dag.add_task("build_artifact", &[], 5, 300, 1);
    dag.add_task("security_scan", &[], 8, 300, 1);
    dag.add_task(
        "deploy_service",
        &["build_artifact", "security_scan"],
        10,
        600,
        2,
    );

    let sorted = dag.topological_sort().unwrap();
    assert_eq!(sorted.len(), 3);
    let deploy_pos = sorted.iter().position(|id| id == "deploy_service").unwrap();
    let build_pos = sorted.iter().position(|id| id == "build_artifact").unwrap();
    let scan_pos = sorted.iter().position(|id| id == "security_scan").unwrap();
    assert!(deploy_pos > build_pos);
    assert!(deploy_pos > scan_pos);
}

#[test]
fn test_dag_cycle_detection() {
    let mut dag = TaskDAG::new();
    dag.add_task("task_a", &["task_c"], 5, 300, 1);
    dag.add_task("task_b", &["task_a"], 5, 300, 1);
    dag.add_task("task_c", &["task_b"], 5, 300, 1);

    let res = dag.topological_sort();
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(!err.cycle_nodes.is_empty());
}

#[test]
fn test_mermaid_and_dot_export() {
    let mut dag = TaskDAG::new();
    dag.add_task("compile", &[], 5, 300, 1);
    dag.add_task("test", &["compile"], 10, 300, 1);

    let mermaid = dag.render_mermaid();
    assert!(mermaid.contains("graph TD"));
    assert!(mermaid.contains("compile --> test"));

    let dot = dag.render_dot();
    assert!(dot.contains("digraph TaskDAG"));
    assert!(dot.contains("\"compile\" -> \"test\""));
}

#[test]
fn test_rate_limiter_and_persistent_kv() {
    let source = "let limiter = rate_limiter_sliding_window(60, 2)\nlet ok1 = consume(limiter, \"client_a\", 1000)\nlet ok2 = consume(limiter, \"client_a\", 1010)\nlet ok3 = consume(limiter, \"client_a\", 1020)\n\nlet kv = persistent_kv(\"test_store\")\nlet dummy = set(kv, \"score\", 99)\nlet val = get(kv, \"score\", 0)\n";
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.scan_tokens().unwrap();
    let mut parser = Parser::new(tokens);
    let stmts = parser.parse().unwrap();
    let mut interpreter = Interpret::default();
    for stmt in stmts {
        interpreter.execute(&stmt);
    }
}
