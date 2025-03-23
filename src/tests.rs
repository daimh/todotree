use super::*;
use std::fmt::Write;
use std::fs;
use std::fs::read_to_string;

#[test]
fn examples() {
    for path in fs::read_dir("examples").unwrap() {
        let md = path.unwrap().path().display().to_string();
        if !md.ends_with(".md") {
            continue;
        }
        for hide in [false, true] {
            for format in vec!["html", "json", "term"] {
                let tree =
                    Tree::new(&md, &mut Vec::<String>::new(), 80, format, hide);
                let mut output = String::new();
                match write!(output, "{}", tree) {
                    Ok(s) => s,
                    Err(e) => panic!("ERR-001: Failed to write '{}'.", e),
                }
                let outdir = match hide {
                    false => "examples/output/",
                    true => "examples/hide/",
                };
                let basefile = md[0..md.len() - 3].replace("examples/", outdir)
                    + "."
                    + format;
                let standard = match read_to_string(&basefile) {
                    Ok(s) => s,
                    Err(e) => panic!("ERR-002: '{}', {}.", basefile, e),
                };
                assert!(
                    standard == output,
                    "ERR-015: md: {}, format: {}, hide: {}.",
                    &md,
                    format,
                    hide,
                );
            }
        }
    }
}

#[test]
#[should_panic(expected = "ERR-019:")]
fn err_019() {
    Tree::new(
        "src/tests/ERR-019.md",
        &Vec::<String>::new(),
        0,
        "term",
        false,
    );
}

#[test]
#[should_panic(expected = "ERR-018:")]
fn err_018() {
    Tree::new(
        "src/tests/ERR-018.md",
        &Vec::<String>::new(),
        0,
        "term",
        false,
    );
}

#[test]
#[should_panic(expected = "ERR-017:")]
fn err_017() {
    Tree::new(
        "src/tests/ERR-017.md",
        &Vec::<String>::new(),
        0,
        "term",
        false,
    );
}

#[test]
#[should_panic(expected = "ERR-016:")]
fn err_016() {
    Tree::new(
        "src/tests/ERR-016.md",
        &Vec::<String>::new(),
        0,
        "term",
        false,
    );
}

#[test]
#[should_panic(expected = "ERR-013:")]
fn err_007() {
    Tree::new(
        "src/tests/ERR-007-1.md",
        &Vec::<String>::new(),
        0,
        "term",
        false,
    );
}

#[test]
#[should_panic(expected = "ERR-007:")]
fn err_007_1() {
    Tree::new(
        "src/tests/ERR-007-1.md",
        &vec![String::from("1")],
        0,
        "term",
        false,
    );
}

#[test]
#[should_panic(expected = "ERR-007:")]
fn err_007_2() {
    Tree::new(
        "src/tests/ERR-007-2.md",
        &vec![String::from("1")],
        0,
        "term",
        true,
    );
}
