use psudo::{
    lexer::Lexer,
    parser::{Interpret, Parser},
};

#[test]
fn test_assertion_primitives() {
    let source = "let r1 = assert(true, \"should pass\")\nlet r2 = assert_eq(10, 10, \"should be equal\")\nlet r3 = assert_ne(10, 20, \"should be not equal\")\nprint \"All assertions passed\"\n";
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
fn test_test_block_parsing() {
    let source = "fn add(a, b)\n    return a + b\n\ntest \"addition test\"\n    let res = add(5, 5)\n    assert_eq(res, 10, \"5 + 5 should be 10\")\n";
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.scan_tokens().unwrap();
    let mut parser = Parser::new(tokens);
    let stmts = parser.parse().unwrap();
    assert_eq!(stmts.len(), 2);
    if let psudo::parser::Statment::TestStatement(name, _) = &stmts[1] {
        assert_eq!(name, "addition test");
    } else {
        panic!("Expected TestStatement");
    }
}
