use getopts::Fail;
use std::fmt;
use std::io;
use std::num::ParseIntError;
mod todo;
pub mod tree;
static ROOT: &str = "/";
static HTMLP: &str = "<p style='font-family: monospace; font-size: 16px; \
    margin: 0px; line-height: 16px'>";

#[derive(PartialEq, Clone)]
pub enum Format {
    Html,
    Json,
    Term,
    Md,
}

#[derive(PartialEq, Debug)]
pub enum Status {
    Completed,
    Pending,
    Actionable,
}
impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Status::Completed => "Completed",
                Status::Pending => "Pending",
                Status::Actionable => "Actionable",
            }
        )
    }
}

#[derive(Debug)]
pub enum TodoError {
    Io(io::Error),
    Input(String),
}
impl fmt::Display for TodoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TodoError::Io(e) => write!(f, "I/O error: {}", e),
            TodoError::Input(msg) => write!(f, "{}", msg),
        }
    }
}
impl From<Fail> for TodoError {
    fn from(err: Fail) -> Self {
        TodoError::Input(err.to_string())
    }
}
impl From<io::Error> for TodoError {
    fn from(err: io::Error) -> Self {
        TodoError::Io(err)
    }
}
impl From<ParseIntError> for TodoError {
    fn from(err: ParseIntError) -> Self {
        TodoError::Input(err.to_string())
    }
}
