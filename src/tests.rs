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
		let inputs = vec![md.clone()];
        for idx in 0..4 {
            let (hide, depth, outdir) = match idx {
                0 => (false, 0, "examples/output/"),
                1 => (true, 0, "examples/hide/"),
                2 => (false, 2, "examples/depth/pos2/"),
                _ => (false, -1, "examples/depth/neg1/"),
            };
            for format in vec!["term", "json", "html", "md"] {
                let result = Tree::new(
                    &inputs,
                    &mut BTreeMap::<String, bool>::new(),
                    &mut Vec::<String>::new(),
                    80,
                    format,
                    hide,
                    depth,
                    "\n",
                    false,
                    true,
                    false,
                    false,
                );
                let tree = match result {
                    Ok(t) => t,
                    Err(e) => panic!("ERR-901: md: {}, e: {}", md, e),
                };
                let mut output = String::new();
                match write!(output, "{}", tree) {
                    Ok(s) => s,
                    Err(e) => panic!("ERR-902: Failed to write '{}'", e),
                }
                let basefile = md[0..md.len() - 3].replace("examples/", outdir)
                    + "."
                    + format;
                let standard = match read_to_string(&basefile) {
                    Ok(s) => s,
                    Err(e) => panic!("ERR-903: '{}', {}", basefile, e),
                };
                assert!(
                    standard == output,
                    "ERR-904: md: {}, format: {}, hide: {}, depth: {}",
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
        if md.len() < 20 {
            panic!("ERR-906: {}", md);
        }
		let inputs = vec![md.clone()];
        let options = md[17..].replace(".md", "");
        let mut auto_add = false;
        let mut owners = BTreeMap::<String, bool>::new();
        for opt in options.split("-") {
            if opt.starts_with('A') {
                auto_add = true;
            } else if opt.starts_with('o') {
                owners = opt[1..]
                    .split(",")
                    .map(|s| (s.to_string(), false))
                    .collect();
            }
        }
        match Tree::new(
            &inputs,
            &mut owners,
            &vec![],
            80,
            "term",
            false,
            0,
            " ",
            false,
            auto_add,
            false,
            false,
        ) {
            Err(e) => assert!(e.msg.starts_with(&md[10..17]), "{}, {}", md, e),
            _ => {
                panic!("ERR-905: {}", md);
            }
        }
    }
}
