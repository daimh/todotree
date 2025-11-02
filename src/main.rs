use getopts::{Matches, Options};
use std::env;
use std::io::{self, ErrorKind, Write};
use std::process::ExitCode;
use std::thread;
use std::time::Duration;
mod todotree;
use inotify::{Inotify, WatchMask};
use todotree::tree::Tree;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let mut opts = Options::new();
    opts.optflag("C", "no-color", "Disable color output.");
    opts.optopt(
        "d",
        "depth",
        "Limit the displayed tree depth. A negative value hides leaf nodes.",
        "DEPTH",
    );
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
    opts.optflag("q", "hide", "Hide completed TODO items.");
    opts.optflag(
        "r",
        "refresh",
        "Automatically refresh the tree when the input file changes.",
    );
    opts.optflag(
        "S", 
        "strict", 
        "Require explicit definitions for todos in dependencies, or raise ERR-003."
    );
    opts.optopt(
        "s",
        "separator",
        "Use STRING instead of \"\\n\" to join multi-line comments.",
        "STRING",
    );
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
    let mdfile = match matches.opt_str("input") {
        Some(x) => x,
        None => String::from("todotree.md"),
    };
    let free = &matches.free;
    let targets = match free.is_empty() {
        true => &vec![],
        false => free,
    };
    if !print_tree(&matches, &mdfile, targets) {
        return ExitCode::FAILURE;
    }
    if matches.opt_present("refresh") {
        match matches.opt_str("format") {
            Some(x) => {
                if x != "term" {
                    return ExitCode::SUCCESS;
                }
            }
            None => (),
        };
        loop {
            let mut inotify = Inotify::init().expect("ERR-017: inotify init");
            match inotify
                .watches()
                .add(&mdfile, WatchMask::MODIFY | WatchMask::CLOSE)
            {
                Ok(_) => (),
                Err(e) => match e.kind() {
                    ErrorKind::NotFound => {
                        thread::sleep(Duration::from_secs(1));
                        continue;
                    }
                    _ => panic!("ERR-019: inotify watch, {}", e),
                },
            }
            let mut buffer = [0; 1024];
            inotify
                .read_events_blocking(&mut buffer)
                .expect("ERR-020: reading events");
            if !print_tree(&matches, &mdfile, targets) {
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}

fn print_tree(matches: &Matches, mdfile: &String, targets: &[String]) -> bool {
    if matches.opt_present("refresh") {
        print!("\x1B[2J\x1B[1;1H");
        io::stdout().flush().expect("ERR-020");
    }
    let format = match matches.opt_str("format") {
        Some(x) => x,
        None => String::new(),
    };
    let depth: i32 = match matches.opt_str("depth") {
        Some(x) => match x.parse() {
            Ok(n) => n,
            Err(e) => {
                eprintln!("{}", e.to_string());
                return false;
            }
        },
        None => 0,
    };
    let separator = match matches.opt_str("separator") {
        Some(x) => x,
        None => String::from("\n"),
    };
    match Tree::new(
        mdfile,
        targets,
        0,
        &format,
        matches.opt_present("hide"),
        depth,
        &separator,
        !matches.opt_present("no-color"),
        matches.opt_present("strict"),
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
Usage: todotree [options] [TODO]...

Visualize tasks as a dependency tree instead of a flat list.
todotree highlights complex relationships and color-codes task statuses,
combining the structure of Makefiles with the readability of Markdown.

Examples: 
    todotree
    todotree -i examples/minimalist.md

Repository:
    https://github.com/daimh/todotree"
        )
    );
}

#[cfg(test)]
mod tests;
