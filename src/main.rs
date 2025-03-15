use std::cell::RefCell;
use std::cmp::{max, min};
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::fmt;
use std::fmt::Write;
use std::fs::read_to_string;
use std::process::ExitCode;
use std::rc::Rc;
use termsize;

static ROOT: &str = "/";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("ERR-001: 'todotree' needs one file and optionally a list of todo");
        return ExitCode::FAILURE;
    }
    let tree = Todo::main(args[1].as_str(), &args[2..], 0);
    print!("{}", tree);
    ExitCode::SUCCESS
}

struct Todo {
    name: String,
    owner: String,
    comment: String,
    done: bool,
    wait: bool,
    dependencies: Vec<String>,
    children: Vec<Rc<RefCell<Todo>>>,
    maxlens: [usize; 3],
}

impl fmt::Display for Todo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_header(f)?;
        let mut connectors: Vec<bool> = Vec::new();
        self.fmt_tree(f, &mut connectors, &self.maxlens)?;
        Ok(())
    }
}

impl Todo {
    fn main(mdfile: &str, targets: &[String], default_screen_width: usize) -> String {
        let map = Todo::readmd(mdfile, targets);
        let mut root = map.get(ROOT).unwrap().borrow_mut();

        let mut path: HashSet<String> = HashSet::new();
        let mut visited: HashSet<String> = HashSet::new();
        let screen_width: usize = match default_screen_width {
            0 => match termsize::get() {
                None => 80,
                Some(x) => x.cols.into(),
            },
            _ => default_screen_width,
        };
        root.build_tree(&map, &mut path, &mut visited, 0, screen_width);
        let mut tree = String::new();
        assert!(write!(tree, "{}", root).is_ok(), "ERR-010: write error");
        tree
    }

    fn readmd(mdfile: &str, params: &[String]) -> HashMap<String, Rc<RefCell<Todo>>> {
        let mut map: HashMap<String, Rc<RefCell<Todo>>> = HashMap::new();
        let mut name = String::new();
        let mut owner = String::new();
        let mut comment = String::new();
        let mut dependencies: Vec<String> = Vec::new();
        let mut targets = params.to_vec();
        let buffer = match read_to_string(mdfile) {
            Ok(md) => md,
            Err(e) => panic!("ERR-009: no such a todotree markdown file '{}'", e),
        };
        for ln in buffer.lines() {
            if ln.starts_with("# ") {
                Todo::create(&mut name, &owner, &comment, &dependencies, &mut map);
                name = ln.get(2..).unwrap().trim().to_string();
                assert!(
                    name != "" && name != ROOT,
                    "ERR-003: '{}' is a reserved Todo name keyword",
                    ROOT
                );
                if targets.len() == 0 {
                    targets.push(name.clone());
                }
                owner = String::new();
                comment = String::new();
                dependencies = Vec::new();
            } else if ln.starts_with("- @ ") {
                owner = ln.get(4..).unwrap().trim().to_string();
            } else if ln.starts_with("- % ") {
                comment = ln.get(4..).unwrap().trim().to_string();
            } else if ln.starts_with("- : ") {
                dependencies = ln
                    .get(4..)
                    .unwrap()
                    .split_whitespace()
                    .map(str::to_string)
                    .collect();
            }
        }
        Todo::create(&mut name, &owner, &comment, &dependencies, &mut map);
        assert!(
            map.len() > 0,
            "ERR-004: The markdown file doesn't have any Todo"
        );
        for todo in &mut *targets {
            assert!(
                map.contains_key(todo),
                "ERR-005: Todo '{}' is missing in the markdown file",
                todo
            );
        }
        map.insert(
            ROOT.to_owned(),
            Rc::new(RefCell::new(Todo {
                name: ROOT.to_owned(),
                owner: "".to_owned(),
                comment: "".to_owned(),
                done: false,
                wait: false,
                dependencies: targets.to_vec(),
                children: Vec::new(),
                maxlens: [0; 3],
            })),
        );
        map
    }

    fn create(
        name: &mut String,
        owner: &String,
        comment: &String,
        dependencies: &Vec<String>,
        map: &mut HashMap<String, Rc<RefCell<Todo>>>,
    ) {
        if name == "" {
            assert!(
                owner == "" && comment == "" && dependencies.len() == 0,
                "ERR-006: Missing '# [TODO]' before '- @', '- :', or '-  %' in the todotree markdown file"
            );
            return;
        }
        let done = name.starts_with("~~");
        if done {
            *name = name.replace("~~", "");
        }
        let todo = Todo {
            name: name.clone(),
            owner: owner.clone(),
            comment: comment.clone(),
            done: done,
            wait: false,
            dependencies: dependencies.clone(),
            children: Vec::new(),
            maxlens: [0; 3],
        };
        map.insert(name.clone(), Rc::new(RefCell::new(todo)));
    }

    fn build_tree(
        &mut self,
        map: &HashMap<String, Rc<RefCell<Todo>>>,
        path: &mut HashSet<String>,
        visited: &mut HashSet<String>,
        depth: usize,
        screen_width: usize,
    ) -> [usize; 3] {
        self.wait = false;
        for dep in &self.dependencies {
            assert!(
                path.insert(dep.clone()),
                "ERR-007: Todo '{}' has a dependency loop",
                self.name
            );
            match map.get(dep) {
                None => panic!("ERR-008: No such a Todo '{}'", dep),
                Some(child) => {
                    if visited.insert(dep.clone()) {
                        self.children.push(Rc::clone(child));
                        let lens = child.borrow_mut().build_tree(
                            map,
                            path,
                            visited,
                            depth + 1,
                            screen_width,
                        );
                        for i in 0..3 {
                            self.maxlens[i] = max(self.maxlens[i], lens[i]);
                        }
                    }
                    self.wait = self.wait || !child.borrow().done;
                }
            };
            path.remove(dep);
        }
        if self.name == ROOT {
            if self.maxlens[1] > 0 {
                self.owner = "OWNER".to_owned();
            }
            if self.maxlens[2] > 0 {
                self.comment = "COMMENT".to_owned();
            }
            assert!(
                screen_width > self.maxlens[0] + self.maxlens[1] + 8,
                "ERR-002: Screen is too narrow for this todotree markdown file"
            );
            self.maxlens[2] = min(
                self.maxlens[2],
                screen_width - self.maxlens[0] - self.maxlens[1] - 8,
            );
        }
        self.maxlens[0] = max(self.maxlens[0], depth * 4 + self.name.len());
        self.maxlens[1] = max(self.maxlens[1], self.owner.len());
        self.maxlens[2] = max(self.maxlens[2], self.comment.len());
        self.maxlens
    }

    fn fmt_tree(
        &self,
        f: &mut fmt::Formatter<'_>,
        connectors: &mut Vec<bool>,
        maxlens: &[usize; 3],
    ) -> fmt::Result {
        for (pos, cn) in connectors.iter().enumerate() {
            if *cn {
                if pos + 1 < connectors.len() {
                    write!(f, "    ")?;
                } else {
                    write!(f, "└── ")?;
                }
            } else if pos + 1 < connectors.len() {
                write!(f, "│   ")?;
            } else {
                write!(f, "├── ")?;
            }
        }
        if self.done {
            // strikethrough
            write!(f, "\x1b\x5b\x39\x6d{}\x1b\x28\x42\x1b\x5b\x6d", self.name)?;
        } else if self.wait {
            write!(f, "{}", self.name)?;
        } else {
            // red
            write!(
                f,
                "\x1b\x5b\x33\x31\x6d{}\x1b\x28\x42\x1b\x5b\x6d",
                self.name
            )?;
        }
        write!(
            f,
            "{}",
            " ".repeat(maxlens[0] - connectors.len() * 4 - self.name.len())
        )?;
        match maxlens[1] + maxlens[2] {
            0 => writeln!(f)?,
            _ => {
                write!(f, " │ ")?;
                write!(f, "{}", self.owner)?;
                write!(f, "{}", " ".repeat(maxlens[1] - self.owner.len()))?;
                if maxlens[1] > 0 && maxlens[2] > 0 {
                    write!(f, " │ ")?;
                }
                self.fmt_comment(f, connectors, maxlens, &mut 0)?;
            }
        }
        for (pos, child) in self.children.iter().enumerate() {
            connectors.push(pos + 1 == self.children.len());
            child.borrow().fmt_tree(f, connectors, maxlens)?;
            connectors.pop();
        }
        Ok(())
    }

    fn fmt_header(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.maxlens[1] + self.maxlens[2] > 0 {
            write!(f, "{}", " ".repeat(self.maxlens[0]))?;
            write!(f, " ┌─")?;
            write!(f, "{}", "─".repeat(self.maxlens[1]))?;
            if self.maxlens[1] > 0 && self.maxlens[2] > 0 {
                write!(f, "─┬─")?;
            }
            write!(f, "{}", "─".repeat(self.maxlens[2]))?;
            writeln!(f, "─┐")?;
        } else {
            write!(f, "")?;
        }
        Ok(())
    }

    fn fmt_comment(
        &self,
        f: &mut fmt::Formatter<'_>,
        connectors: &Vec<bool>,
        maxlens: &[usize; 3],
        start: &mut usize,
    ) -> fmt::Result {
        let mut last = self.children.len() == 0;
        if last {
            for b in connectors {
                last = *b;
                if !last {
                    break;
                }
            }
        }
        loop {
            let slen = min(self.comment.len() - *start, maxlens[2]);
            write!(f, "{}", &self.comment[*start..*start + slen])?;
            write!(f, "{}", " ".repeat(maxlens[2] - slen))?;
            writeln!(f, " │")?;
            *start = *start + slen;
            for b in connectors {
                match *b {
                    true => write!(f, " ")?,
                    false => write!(f, "│")?,
                };
                write!(f, "   ")?;
            }
            match self.children.len() {
                0 => write!(f, "    "),
                _ => write!(f, "│   "),
            }?;
            write!(f, "{}", " ".repeat(maxlens[0] - 4 - connectors.len() * 4))?;
            if *start < self.comment.len() {
                write!(f, " │ ")?;
                write!(f, "{}", " ".repeat(maxlens[1]))?;
                if maxlens[1] > 0 && maxlens[2] > 0 {
                    write!(f, " │ ")?;
                }
            } else {
                match last {
                    false => write!(f, " ├─"),
                    true => write!(f, " └─"),
                }?;
                write!(f, "{}", "─".repeat(maxlens[1]))?;
                if maxlens[1] > 0 && maxlens[2] > 0 {
                    match last {
                        false => write!(f, "─┼─"),
                        true => write!(f, "─┴─"),
                    }?;
                }
                write!(f, "{}", "─".repeat(maxlens[2]))?;
                match last {
                    false => writeln!(f, "─┤"),
                    true => writeln!(f, "─┘"),
                }?;
                break;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_main() {
        for path in fs::read_dir("examples").unwrap() {
            let md = path.unwrap().path().display().to_string();
            if !md.ends_with(".md") {
                continue;
            }
            let output = Todo::main(&md, &mut Vec::<String>::new(), 80);
            let txt = md.replace(".md", ".txt");
            let standard = match read_to_string(txt) {
                Ok(s) => s,
                Err(e) => panic!("ERR-009: no such a todotree markdown file '{}'", e),
            };
            assert_eq!(standard, output, "");
        }
    }
    #[test]
    #[should_panic(expected = "ERR-007: Todo '1' has a dependency loop")]
    fn test_loop1() {
        Todo::main("src/tests/loop1.md", &mut Vec::<String>::new(), 0);
    }

    #[test]
    #[should_panic(expected = "ERR-007: Todo '3' has a dependency loop")]
    fn test_loop2() {
        Todo::main("src/tests/loop2.md", &mut Vec::<String>::new(), 0);
    }
}
