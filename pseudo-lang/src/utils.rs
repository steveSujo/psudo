//this module is for utillity of the cli
use std::error::Error;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

// Gate ANSI escape sequences on interactive TTY + NO_COLOR so piped LLM orchestrators
// and scripts never burn tokens or choke on raw escape codes.
pub fn use_ansi_stdout() -> bool {
    std::io::IsTerminal::is_terminal(&std::io::stdout()) && std::env::var_os("NO_COLOR").is_none()
}

pub fn use_ansi_stderr() -> bool {
    std::io::IsTerminal::is_terminal(&std::io::stderr()) && std::env::var_os("NO_COLOR").is_none()
}

pub fn paint(text: &str, code: &str, tty: bool) -> String {
    if tty {
        format!("\x1b[{}m{}\x1b[0m", code, text)
    } else {
        text.to_string()
    }
}

pub fn cyan_bold(text: &str) -> String {
    paint(text, "1;36", use_ansi_stdout())
}

pub fn green_bold(text: &str) -> String {
    paint(text, "1;32", use_ansi_stdout())
}

pub fn yellow(text: &str) -> String {
    paint(text, "33", use_ansi_stdout())
}

pub fn yellow_err(text: &str) -> String {
    paint(text, "33", use_ansi_stderr())
}

pub fn red_bold(text: &str) -> String {
    paint(text, "1;31", use_ansi_stderr())
}

pub fn dim(text: &str) -> String {
    paint(text, "2", use_ansi_stdout())
}
