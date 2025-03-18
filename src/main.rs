use getopts::Options;
use std::env;
use std::process::ExitCode;
mod todotree;
use todotree::tree::Tree;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let mut opts = Options::new();
    opts.optopt(
        "i",
        "input",
        "use MARKDOWN file instead of 'todotree.md' as input",
        "INPUT",
    );
    opts.optopt(
        "o",
        "format",
        "set output format to 'html', 'json', or 'term' (by default)",
        "FORMAT",
    );
    opts.optflag("n", "nodone", "hide todos that are done");
    opts.optflag("h", "help", "print this help menu");
    opts.optflag("", "version", "print version");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            eprintln!("{}", f.to_string());
            return ExitCode::FAILURE;
        }
    };
    if matches.opt_present("version") {
        print_version();
        return ExitCode::SUCCESS;
    }
    if matches.opt_present("h") {
        print_usage(&opts);
        return ExitCode::SUCCESS;
    }
    let input = match matches.opt_str("i") {
        Some(x) => x,
        None => String::from("todotree.md"),
    };
    let format = match matches.opt_str("o") {
        Some(x) => x,
        None => String::new(),
    };
    let free = &matches.free;
    let targets = match free.is_empty() {
        true => &vec![],
        false => free,
    };
    let tree = Tree::new(
        &input,
        targets,
        matches.opt_present("n"),
        0,
        format.as_str(),
    );
    print!("{}", tree);
    ExitCode::SUCCESS
}

fn print_version() {
    println!("20250317");
}

fn print_usage(opts: &Options) {
    print!(
        "{}",
        opts.usage(
            "Usage: todotree [options] [TODO]...\n\
            Display todos with a dependency tree. Highlight ongoing ones \n\
            with red, or finished ones with strikethrough. Support \n\
            terminal, html and json output format."
        )
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
            for format in vec!["html", "json", "term"] {
                let tree = Tree::new(
                    &md,
                    &mut Vec::<String>::new(),
                    false,
                    80,
                    format,
                );
                let mut output = String::new();
                match write!(output, "{}", tree) {
                    Ok(s) => s,
                    Err(e) => panic!("ERR-001: failed to write '{}'", e),
                }
                let basefile = md[0..md.len() - 3]
                    .replace("examples/", "examples/output/")
                    + "."
                    + format;
                let standard = match read_to_string(&basefile) {
                    Ok(s) => s,
                    Err(e) => panic!("ERR-002: '{}', {}", basefile, e),
                };
                assert!(
                    standard == output,
                    "ERR-015: md: {}, format: {}",
                    &md,
                    format
                );
            }
        }
    }

    #[test]
    #[should_panic(expected = "ERR-007: Todo '1' has a dependency loop")]
    fn test_loop1() {
        Tree::new(
            "src/tests/loop1.md",
            &vec![String::from("1")],
            false,
            0,
            "term",
        );
    }

    #[test]
    #[should_panic(expected = "ERR-007: Todo '3' has a dependency loop")]
    fn test_loop2() {
        Tree::new(
            "src/tests/loop2.md",
            &vec![String::from("1")],
            false,
            0,
            "term",
        );
    }
}
