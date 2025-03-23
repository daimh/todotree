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
    let free = &matches.free;
    let targets = match free.is_empty() {
        true => &vec![],
        false => free,
    };
    let tree = Tree::new(
        &input,
        targets,
        0,
        format.as_str(),
        matches.opt_present("n"),
    );
    print!("{}", tree);
    ExitCode::SUCCESS
}

fn print_version() {
    println!("20250319");
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
mod tests;
