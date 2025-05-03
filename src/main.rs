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
        "MARKDOWN",
    );
    opts.optopt(
        "o",
        "format",
        "set output FORMAT to 'html', 'json', or 'term' (by default)",
        "FORMAT",
    );
    opts.optopt(
        "d",
        "depth",
        "max display of the tree. Negative int removes the leaf nodes",
        "DEPTH",
    );
    opts.optopt(
        "s",
        "separator",
        "use STRING instead of \"\\n\" to join multiple lines of comments",
        "STRING",
    );
    opts.optflag("n", "hide", "hide todos that are completed");
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
    let depth: i32 = match matches.opt_str("d") {
        Some(x) => match x.parse() {
            Ok(n) => n,
            Err(e) => {
                eprintln!("{}", e.to_string());
                return ExitCode::FAILURE;
            }
        },
        None => 0,
    };
    let free = &matches.free;
    let targets = match free.is_empty() {
        true => &vec![],
        false => free,
    };
    let separator = match matches.opt_str("s") {
        Some(x) => x,
        None => String::from("\n"),
    };
    match Tree::new(
        &input,
        targets,
        0,
        format.as_str(),
        matches.opt_present("n"),
        depth,
        separator.as_str(),
    ) {
        Ok(tree) => {
            print!("{}", tree);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}", e);
            ExitCode::FAILURE
        }
    }
}

fn print_version() {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    const LICENSE: &str = env!("CARGO_PKG_LICENSE");
    const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");
    println!(
        "todotree {}, {} License, Copyright (c) {}",
        VERSION, LICENSE, AUTHOR
    );
}

fn print_usage(opts: &Options) {
    print!(
        "{}",
        opts.usage(
            "Usage: todotree [options] [TODO]...\n\
			Visualize tasks as a dependency tree rather than a flat list, highlighting complex relationships and color-coding their statuses. Inspired by the structure of Makefiles and the readability of Markdown.
            \n\
            Repo: https://github.com/daimh/todotree"
        )
    );
}

#[cfg(test)]
mod tests;
