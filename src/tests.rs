use super::*;
use std::fmt::Write;
use std::fs::{read_dir, read_to_string};

#[test]
fn examples() {
    for path in read_dir("examples").unwrap() {
        let md = path.unwrap().path().display().to_string();
        if !md.ends_with(".md") {
            continue;
        }
        for format in vec!["term", "json", "html"] {
            for idx in 0..4 {
                let (hide, depth, outdir) = match idx {
                    0 => (false, 0, "examples/output/"),
                    1 => (true, 0, "examples/hide/"),
                    2 => (false, 2, "examples/depth/pos2/"),
                    _ => (false, -1, "examples/depth/neg1/"),
                };
                let result = Tree::new(
                    &md,
                    &mut Vec::<String>::new(),
                    80,
                    format,
                    hide,
                    depth,
                    "\n",
                );
                let tree = match result {
                    Ok(t) => t,
                    Err(e) => panic!("ERR-901: md: {}, e: {}", md, e),
                };
                let mut output = String::new();
                match write!(output, "{}", tree) {
                    Ok(s) => s,
                    Err(e) => panic!("ERR-902: Failed to write '{}'.", e),
                }
                let basefile = md[0..md.len() - 3].replace("examples/", outdir)
                    + "."
                    + format;
                let standard = match read_to_string(&basefile) {
                    Ok(s) => s,
                    Err(e) => panic!("ERR-903: '{}', {}.", basefile, e),
                };
                assert!(
                    standard == output,
                    "ERR-904: md: {}, format: {}, hide: {}, depth: {}.",
                    &md,
                    format,
                    hide,
                    depth,
                );
            }
        }
    }
}

#[test]
fn errors() {
    for path in read_dir("src/tests/").unwrap() {
        let md = path.unwrap().path().display().to_string();
        if !md.ends_with(".md") || !md.starts_with("src/tests/ERR-") {
            continue;
        }
        let target = match md.len() {
            20 => Vec::<String>::new(),
            _ => vec![md[18..].replace(".md", ""); 1],
        };
        match Tree::new(&md, &target, 0, "term", false, 0, " ") {
            Err(e) => assert!(e.msg.starts_with(&md[10..17]), "{}, {}", md, e),
            _ => panic!("ERR-905"),
        }
    }
}
