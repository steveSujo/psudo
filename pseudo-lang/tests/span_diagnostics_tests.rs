use psudo::diagnostics::{Diagnostic, ErrorCode};
use psudo::lexer::Lexer;

#[test]
fn test_failing_lexer_span_and_diagnostic_code() {
    let mut lexer = Lexer::new("# invalid comment syntax".to_string());
    let res = lexer.scan_tokens();
    assert!(
        res.is_err(),
        "Expected lexer error for unrecognized character '#'"
    );

    let err = res.unwrap_err();
    // Verify span and diagnostic error code E001
    assert_eq!(err.code(), ErrorCode::E001UnrecognizedChar);
    assert_eq!(err.span.start_line, 1);
    assert_eq!(err.span.start_col, 1);

    // Verify JSON diagnostic rendering
    let diagnostic = Diagnostic::from_lexer_error(&err, "src/main.psudo");
    let json_str = diagnostic.to_json();
    assert!(json_str.contains("\"code\":\"E001\""));
    assert!(json_str.contains("\"line\":1"));
    assert!(json_str.contains("\"column\":1"));
}
