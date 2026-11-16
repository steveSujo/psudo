pub mod blackbox;
pub mod codegen;
pub mod config;
pub mod context;
pub mod crdt;
pub mod dag;
pub mod diagnostics;
pub mod embedded_model;
pub mod error_handler;
pub mod fs;
pub mod lexer;
pub mod lsp;
pub mod mcp;
pub mod ownership;
pub mod parser;
pub mod projector;
pub mod resolver;
pub mod user_docs;
pub mod utils;
pub mod vfs;
pub mod zettel;

pub mod contract_gen;
pub mod pir_parser;

pub mod tokens {
    use serde::{Deserialize, Serialize};
    use std::fmt::{self, Display};
    #[derive(Debug, Clone)]
    pub enum LiteralType {
        String(String),
        Number(f32),
        Bool(bool),
        Null,
    }

    impl LiteralType {
        pub fn print(&self) -> String {
            match self {
                LiteralType::String(x) => x.to_string(),
                LiteralType::Number(x) => x.to_string(),
                LiteralType::Bool(b) => b.to_string(),
                LiteralType::Null => "null".to_string(),
            }
        }
    }

    #[derive(Debug, PartialEq, Clone)]
    pub enum TokenType {
        //single char
        LeftPara,
        RightPara,
        LeftBrace,
        RightBrace,
        LeftBracket,
        RightBracket,
        COMMA,
        COLON,
        DOT,
        MINUS,
        PLUS,
        SLASH,
        STAR,

        BANG,
        BangEqual,
        EQUAL,
        EqualEqual,
        GREATER,
        GreaterEqual,
        LESS,
        LessEqual,

        IDENTIFIER,
        STRING,
        NUMBER,

        AND,
        CLASS,
        ELSE,
        FALSE,
        FN,
        FOR,
        IF,
        NIL,
        OR,
        PRINT,
        RETURN,
        SUPER,
        THIS,
        TRUE,
        LET,
        WHILE,
        TEST,
        ARROW,

        // Indentation-based block structure
        NEWLINE,
        INDENT,
        DEDENT,

        EOF,
    }
    // impl Display for TokenType {
    //     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    //         write!(f, "{}", self)
    //     }
    // }
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Span {
        pub line: usize,
        pub col: usize,
        pub offset: usize,
        pub len: usize,
        pub start_line: usize,
        pub start_col: usize,
        pub end_line: usize,
        pub end_col: usize,
    }

    impl Span {
        pub fn new(start_line: usize, start_col: usize, end_line: usize, end_col: usize) -> Self {
            Span {
                line: start_line,
                col: start_col,
                offset: 0,
                len: 1,
                start_line,
                start_col,
                end_line,
                end_col,
            }
        }
        pub fn with_details(line: usize, col: usize, offset: usize, len: usize) -> Self {
            Span {
                line,
                col,
                offset,
                len,
                start_line: line,
                start_col: col,
                end_line: line,
                end_col: col + len,
            }
        }
        pub fn single(line: usize, col: usize) -> Self {
            Span {
                line,
                col,
                offset: 0,
                len: 1,
                start_line: line,
                start_col: col,
                end_line: line,
                end_col: col,
            }
        }
    }

    impl Default for Span {
        fn default() -> Self {
            Span {
                line: 1,
                col: 1,
                offset: 0,
                len: 1,
                start_line: 1,
                start_col: 1,
                end_line: 1,
                end_col: 1,
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct Token {
        pub token_type: TokenType,
        pub text: String,
        pub literal: Option<LiteralType>,
        pub line: usize,
        pub span: Span,
    }

    impl Token {
        pub fn new(
            token_type: TokenType,
            text: String,
            literal: Option<LiteralType>,
            line: usize,
        ) -> Self {
            Token {
                token_type,
                text,
                literal,
                line,
                span: Span::default(),
            }
        }

        pub fn with_span(
            token_type: TokenType,
            text: String,
            literal: Option<LiteralType>,
            span: Span,
        ) -> Self {
            Token {
                token_type,
                text,
                literal,
                line: span.start_line,
                span,
            }
        }
    }

    impl Display for Token {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(
                f,
                "{:#?} {} {:#?}",
                self.token_type, self.text, self.literal
            )
        }
    }
}
