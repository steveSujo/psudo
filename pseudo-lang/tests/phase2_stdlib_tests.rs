use psudo::{
    lexer::Lexer,
    ownership::{BorrowChecker, CheckedEnvironment},
    parser::{CheckedInterpret, Interpret, Parser},
    vfs::WorkspaceCapsule,
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

#[test]
fn test_fstring_interpolation() {
    let source = "let name = \"Alice\"\nlet count = 3\nlet msg = f\"Hello {name}, items: {count}!\"\nprint msg\n";
    run_test(source);
}

#[test]
fn test_mutable_while_loop() {
    let source =
        "let sum = 0\nlet i = 1\nwhile i <= 5\n    sum = sum + i\n    i = i + 1\nprint sum\n";
    run_test(source);
}

#[test]
fn test_math_primitives() {
    let source = "let s = sqrt(16)\nlet p = pow(2, 3)\nlet a = abs(-10)\nlet m = min(5, 8)\nlet mx = max(5, 8)\nprint s\nprint p\nprint a\nprint m\nprint mx\n";
    run_test(source);
}

#[test]
fn test_time_and_deque() {
    let source = "let t = time_now_sec()\nprint t\nlet d = deque_new(10)\nlet d2 = push_front(d, 42)\nlet front_val = pop_front(d2)\nprint front_val\n";
    run_test(source);
}

#[test]
fn test_currency_and_json() {
    let source = "let curr = fmt_currency(123.456, \"$\", 2)\nprint curr\n";
    run_test(source);
}
