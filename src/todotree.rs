mod todo;
pub mod tree;
static ROOT: &str = "/";
static HTMLP: &str = "<p style='font-family: monospace; font-size: 16px; \
	margin: 0px; line-height: 16px'>";

#[derive(PartialEq)]
pub enum Format {
    Html,
    Json,
    Term,
}

#[derive(PartialEq, Debug)]
pub enum Status {
    Completed,
    Pending,
    Actionable,
}
