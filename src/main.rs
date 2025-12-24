use getopts::{Matches, Options};
use std::collections::BTreeMap;
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
    opts.optflag(
        "A",
        "auto-add",
        "Auto-add missing TODO definitions from dependencies.",
    );
    opts.optflag("C", "no-color", "Disable color output.");
    opts.optflag("M", "hide-comment", "Hide comment column.");
    opts.optflag("O", "hide-owner", "Hide owner column.");
    opts.optflag("R", "reverse", "Reverse the tree order.");
    opts.optopt(
        "d",
        "depth",
        "Limit tree depth. A negative value hides leaf nodes.",
        "N",
    );
    opts.optmulti(
        "i",
        "input",
        "Read TODOs from FILE (default:  'todotree.md'). \
			May be specified multiple times.",
        "FILE",
    );
    opts.optopt(
        "f",
        "format",
        "Output format: term | md | html | json (default: term).",
        "FORMAT",
    );
    opts.optmulti(
        "o",
        "owner",
        "Show only TODOs owned by OWNER. May be specified multiple times.",
        "OWNER",
    );
    opts.optflag("q", "hide-done", "Hide completed TODOs.");
    opts.optflag("r", "refresh", "Auto-refresh when input file changes.");
    opts.optopt(
        "s",
        "separator",
        "Join multi-line comments with STRING (default: \"\\n\").",
        "STR",
    );
    opts.optflag("h", "help", "Show this help and exit.");
    opts.optflag("", "version", "Show version information and exit.");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            eprintln!("ERR-023: {}", f.to_string());
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
    let mut inputs = matches.opt_strs("input");
    if inputs.len() == 0 {
        inputs = vec!["todotree.md".to_string()];
    }
    let mut owners: BTreeMap<String, bool> = matches
        .opt_strs("owner")
        .into_iter()
        .map(|s| (s.to_string(), false))
        .collect::<BTreeMap<String, bool>>();
    let free = &matches.free;
    let targets = if free.is_empty() { &vec![] } else { free };
    if !print_tree(&matches, &inputs, &mut owners, targets) {
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
            for mdfile in &inputs {
                if let Err(e) = inotify
                    .watches()
                    .add(&mdfile, WatchMask::MODIFY | WatchMask::CLOSE)
                {
                    match e.kind() {
                        ErrorKind::NotFound => {
                            thread::sleep(Duration::from_secs(1));
                            continue;
                        }
                        _ => panic!("ERR-019: inotify watch, {}", e),
                    }
                }
            }
            let mut buffer = [0u8; 4096];
            inotify
                .read_events_blocking(&mut buffer)
                .expect("ERR-020: reading events");
            print_tree(&matches, &inputs, &mut owners, targets);
        }
    }
    ExitCode::SUCCESS
}

fn print_tree(
    matches: &Matches,
    inputs: &Vec<String>,
    owners: &mut BTreeMap<String, bool>,
    targets: &[String],
) -> bool {
    if matches.opt_present("refresh") {
        print!("\x1B[2J\x1B[1;1H");
        io::stdout().flush().expect("ERR-021: refresh console");
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
        inputs,
        owners,
        targets,
        0,
        &format,
        matches.opt_present("hide-done"),
        depth,
        &separator,
        matches.opt_present("no-color"),
        matches.opt_present("auto-add"),
        matches.opt_present("hide-comment"),
        matches.opt_present("hide-owner"),
        matches.opt_present("reverse"),
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

Description:
Visualizes tasks as a dependency tree instead of a flat list.
Highlights dependencies, color-codes task status, and uses a Markdown-like
format.

Repository: https://github.com/daimh/todotree

Examples:
    cd examples
    todotree
    todotree -R
    todotree -o Avery -o Dad
    todotree -i todotree.md
    todotree -i todotree.md lawn
    todotree -A -i minimalist.md
"
        )
    );
}

#[cfg(test)]
mod tests;
