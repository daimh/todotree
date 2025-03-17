use getopts::Options;
use std::env;
use std::process::ExitCode;
mod todotree;
use todotree::tree::Tree;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let mut opts = Options::new();
    opts.optopt(
        "f",
        "format",
        "set output format to 'html', 'json', or 'term'(default)",
        "FORMAT",
    );
    opts.optflag("h", "help", "print this help menu");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            eprintln!("{}", f.to_string());
            return ExitCode::FAILURE;
        }
    };
    if matches.opt_present("h") {
        print_usage(&opts);
        return ExitCode::SUCCESS;
    }
    let input = &matches.free;
    if input.is_empty() {
        print_usage(&opts);
        return ExitCode::FAILURE;
    };
    let fmt_s = match matches.opt_str("f") {
        Some(x) => x,
        None => String::new(),
    };
    let tree = Tree::new(input[0].as_str(), &input[1..], 0, fmt_s.as_str());
    print!("{}", tree);
    ExitCode::SUCCESS
}

fn print_usage(opts: &Options) {
    print!(
        "{}",
        opts.usage("Usage: todotree [options] MARKDOWN [TODO]...")
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write;
    use std::fs;
    use std::fs::read_to_string;

    #[test]
    fn test_main() {
        for path in fs::read_dir("examples").unwrap() {
            let md = path.unwrap().path().display().to_string();
            if !md.ends_with(".md") {
                continue;
            }
            for fmt_s in vec!["html", "json", "term"] {
                let tree = Tree::new(&md, &mut Vec::<String>::new(), 80, fmt_s);
                let mut output = String::new();
                match write!(output, "{}", tree) {
                    Ok(s) => s,
                    Err(e) => panic!("ERR-001: failed to write '{}'", e),
                }
                let file =
                    md[0..md.len() - 3].replace("examples/", "examples/output/") + "." + fmt_s;
                let standard = match read_to_string(&file) {
                    Ok(s) => s,
                    Err(e) => panic!("ERR-002: no such a todotree markdown file '{}'", e),
                };
                assert_eq!(standard, output, "md: {}, format: {}", &md, fmt_s);
            }
        }
    }

    #[test]
    #[should_panic(expected = "ERR-007: Todo '1' has a dependency loop")]
    fn test_loop1() {
        Tree::new("src/tests/loop1.md", &mut Vec::<String>::new(), 0, "term");
    }

    #[test]
    #[should_panic(expected = "ERR-007: Todo '3' has a dependency loop")]
    fn test_loop2() {
        Tree::new("src/tests/loop2.md", &mut Vec::<String>::new(), 0, "term");
    }
}
