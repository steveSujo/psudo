use crate::tokens::Span;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    E001UnrecognizedChar,
    E002IndentationError,
    E003UnclosedString,
    E004UnresolvedSymbol,
    E005BorrowConflict,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::E001UnrecognizedChar => "E001",
            ErrorCode::E002IndentationError => "E002",
            ErrorCode::E003UnclosedString => "E003",
            ErrorCode::E004UnresolvedSymbol => "E004",
            ErrorCode::E005BorrowConflict => "E005",
        }
    }

    pub fn help_message(&self) -> &'static str {
        match self {
            ErrorCode::E001UnrecognizedChar => {
                "Pseudo uses '//' for single-line comments and '/* ... */' for block comments."
            }
            ErrorCode::E002IndentationError => {
                "Pseudo uses consistent 4-space Python-style indentation for blocks."
            }
            ErrorCode::E003UnclosedString => {
                "Ensure string literals are terminated with closing quotes."
            }
            ErrorCode::E004UnresolvedSymbol => {
                "Declare variables with 'let' or functions with 'fn' before referencing them."
            }
            ErrorCode::E005BorrowConflict => {
                "Check ownership rules: variables cannot be mutated while borrowed."
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub level: String,
    pub code: String,
    pub message: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub help: String,
}

#[derive(Debug, Clone)]
pub struct LexerErrorInfo {
    pub code: ErrorCode,
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for LexerErrorInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: line {}, col {}",
            self.message, self.span.line, self.span.col
        )
    }
}

impl std::error::Error for LexerErrorInfo {}

impl LexerErrorInfo {
    pub fn code(&self) -> ErrorCode {
        self.code
    }
}

impl Diagnostic {
    pub fn from_lexer_error(err: &LexerErrorInfo, file: &str) -> Self {
        Diagnostic {
            level: "error".to_string(),
            code: err.code.as_str().to_string(),
            message: err.message.clone(),
            file: file.to_string(),
            line: err.span.line,
            column: err.span.col,
            help: err.code.help_message().to_string(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn render_terminal(&self, source: &str) -> String {
        let lines: Vec<&str> = source.lines().collect();
        let target_line_idx = self.line.saturating_sub(1);
        let src_line = lines.get(target_line_idx).unwrap_or(&"");
        let padding = " ".repeat(self.column.saturating_sub(1));
        let caret = "^";

        format!(
            "error[{}]: {}\n --> {}:{}:{}\n  |\n{} | {}\n  | {}{} unexpected character\n  =\n  = help: {}\n",
            self.code, self.message, self.file, self.line, self.column,
            self.line, src_line, padding, caret, self.help
        )
    }
}
