use getopts::Options;
use std::env;
use std::io::ErrorKind;
use std::process::ExitCode;
use std::thread;
use std::time::Duration;
mod todotree;
use inotify::{Inotify, WatchMask};
use todotree::tree::Tree;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let mut opts = Options::new();
    opts.optopt(
        "i",
        "input",
        "Use MARKDOWN file as input (default:  'todotree.md')",
        "MARKDOWN",
    );
    opts.optopt(
        "o",
        "format",
        "Set output format to 'html', 'json', 'md', or 'term' (default: 'term')",
        "FORMAT",
    );
    opts.optopt(
        "d",
        "depth",
        "Limit the displayed tree depth. A negative value hides leaf nodes.",
        "DEPTH",
    );
    opts.optopt(
        "s",
        "separator",
        "Use STRING instead of \"\\n\" to join multi-line comments.",
        "STRING",
    );
    opts.optflag("q", "hide", "Hide completed TODO items.");
    opts.optflag(
        "r",
        "refresh",
        "Automatically refresh the tree when the input file changes.",
    );
    opts.optflag("C", "no-color", "Disable color output.");
    opts.optflag("h", "help", "Show this help message and exit.");
    opts.optflag("", "version", "Show version information and exit.");
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
    if matches.opt_present("help") {
        print_usage(&opts);
        return ExitCode::SUCCESS;
    }
    let input = match matches.opt_str("input") {
        Some(x) => x,
        None => String::from("todotree.md"),
    };
    let format = match matches.opt_str("format") {
        Some(x) => x,
        None => String::new(),
    };
    let depth: i32 = match matches.opt_str("depth") {
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
    let separator = match matches.opt_str("separator") {
        Some(x) => x,
        None => String::from("\n"),
    };
    if matches.opt_present("refresh") {
        print!("\x1B[2J\x1B[1;1H");
    }
    if !print_tree(
        &input,
        targets,
        0,
        format.as_str(),
        matches.opt_present("hide"),
        depth,
        separator.as_str(),
        !matches.opt_present("no-color"),
    ) {
        return ExitCode::FAILURE;
    }
    if matches.opt_present("refresh") {
        loop {
            let mut inotify = Inotify::init()
                .expect("Error while initializing inotify instance");
            match inotify
                .watches()
                .add(&input, WatchMask::MODIFY | WatchMask::CLOSE)
            {
                Ok(_) => (),
                Err(e) => {
                    println!("{}", e);
                    match e.kind() {
                        ErrorKind::NotFound => {
                            thread::sleep(Duration::from_secs(1));
                            continue;
                        }
                        _ => break,
                    }
                }
            }
            let mut buffer = [0; 1024];
            inotify
                .read_events_blocking(&mut buffer)
                .expect("Error while reading events");
            print!("\x1B[2J\x1B[1;1H");
            if !print_tree(
                &input,
                targets,
                0,
                format.as_str(),
                matches.opt_present("hide"),
                depth,
                separator.as_str(),
                !matches.opt_present("no-color"),
            ) {
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}

fn print_tree(
    mdfile: &str,
    targets: &[String],
    term_width: usize,
    format: &str,
    hide: bool,
    dpth_limit: i32,
    separator: &str,
    color: bool,
) -> bool {
    match Tree::new(
        mdfile, targets, term_width, format, hide, dpth_limit, separator, color,
    ) {
        Ok(tree) => {
            print!("{}", tree);
            return true;
        }
        Err(e) => {
            eprintln!("{}", e);
            return false;
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
            "\
Usage: todotree [options] [TODO]...\n\n\
Visualize tasks as a dependency tree instead of a flat list.\n\
todotree highlights complex relationships and color-codes task statuses,\n\
combining the structure of Makefiles with the readability of Markdown.\n\n\
Repo: https://github.com/daimh/todotree"
        )
    );
}

#[cfg(test)]
mod tests;
