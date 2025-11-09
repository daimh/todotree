use std::error::Error;
use std::fmt;
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
pub struct TodoError {
    pub msg: String,
}
impl Error for TodoError {}
impl fmt::Display for TodoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}
