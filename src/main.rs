use getopts::{Matches, Options};
use std::collections::BTreeMap;
use std::env;
use std::fs::OpenOptions;
use std::io::{self, ErrorKind, Write};
use std::thread;
use std::time::Duration;
mod todotree;
use inotify::{Inotify, WatchMask};
use todotree::{TodoError, tree::Tree};

fn main() -> Result<(), TodoError> {
    let args: Vec<String> = env::args().collect();
    let mut opts = Options::new();
    opts.optflag(
        "A",
        "auto-add",
        "Auto-add missing TODO definitions from dependencies.",
    );
    opts.optflag("C", "no-color", "Disable color output.");
    opts.optflag(
        "E",
        "example",
        "Create a sample todotree.md in the current directory.",
    );
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
    let matches = opts.parse(&args[1..])?;
    if matches.opt_present("version") {
        return print_version();
    }
    if matches.opt_present("help") {
        return print_usage(&opts);
    }
    if matches.opt_present("example") {
        let path = "todotree.md"; // file path
        let content = "\
# lawn
- @ Avery
- : mower
- % at noon
- % mow the lawn

# ~mower
- @ Brody
- % test the mower
";
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true) // <-- fail if the file exists
            .open(path)?;
        file.write_all(content.as_bytes())?;
        println!("Created todotree.md");
        return Ok(());
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
    print_tree(&matches, &inputs, &mut owners, targets)?;
    if matches.opt_present("refresh") {
        match matches.opt_str("format") {
            Some(x) => {
                if x != "term" {
                    return Ok(());
                }
            }
            None => (),
        };
        loop {
            let mut inotify = Inotify::init()?;
            for mdfile in &inputs {
                if let Err(e) = inotify
                    .watches()
                    .add(&mdfile, WatchMask::MODIFY | WatchMask::CLOSE)
                {
                    if e.kind() == ErrorKind::NotFound {
                        thread::sleep(Duration::from_secs(1));
                    } else {
                        return Err(TodoError::Input(format!(
                            "ERR-019: Inotify, {}",
                            e
                        )));
                    }
                }
            }
            let mut buffer = [0u8; 4096];
            inotify.read_events_blocking(&mut buffer)?;
            if let Err(e) = print_tree(&matches, &inputs, &mut owners, targets)
            {
                println!("{}", e);
            }
        }
    }
    Ok(())
}

fn print_tree(
    matches: &Matches,
    inputs: &Vec<String>,
    owners: &mut BTreeMap<String, bool>,
    targets: &[String],
) -> Result<(), TodoError> {
    if matches.opt_present("refresh") {
        print!("\x1B[2J\x1B[1;1H");
        io::stdout().flush()?;
    }
    let format = match matches.opt_str("format") {
        Some(x) => x,
        None => String::new(),
    };
    let depth: i32 = match matches.opt_str("depth") {
        Some(x) => x.parse()?,
        None => 0,
    };
    let separator = match matches.opt_str("separator") {
        Some(x) => x,
        None => "\n".to_string(),
    };
    let tree = Tree::new(
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
    )?;
    print!("{}", tree);
    Ok(())
}

fn print_version() -> Result<(), TodoError> {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    const LICENSE: &str = env!("CARGO_PKG_LICENSE");
    const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");
    println!(
        "todotree {}, {} License, Copyright (c) {}",
        VERSION, LICENSE, AUTHOR
    );
    Ok(())
}

fn print_usage(opts: &Options) -> Result<(), TodoError> {
    print!(
        "{}",
        opts.usage(
            "\
Usage: todotree [options] [TODO]...

Description:
Visualizes tasks as a dependency tree instead of a flat list.
Highlights dependencies, color-codes task status, and uses a Markdown
format as input.

Repository: https://github.com/daimh/todotree

Examples:
    todotree -E; ls -l todotree.md
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
    Ok(())
}

#[cfg(test)]
mod tests;
