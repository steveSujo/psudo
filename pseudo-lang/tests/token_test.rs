use psudo::{
    lexer::Lexer,
    parser::{Interpret, Parser},
};

#[test]
fn test_interpret() {
    let source = "let s = sqrt(16)\nprint s\n";
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.scan_tokens().unwrap();
    let mut parser = Parser::new(tokens);
    let stmts = parser.parse().unwrap();
    let mut interpreter = Interpret::default();
    for stmt in stmts {
        interpreter.execute(&stmt);
    }
}
