use colored::Colorize;
use std::cell::RefCell;
use std::cmp;
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::fs::read_to_string;
use std::rc::Rc;
use termsize;

static ROOT: &str = "/";

fn main() {
    let args: Vec<String> = env::args().collect();
    assert! {
        args.len() >= 2,
        "ERR-001: 'todotree' needs one file and optionally a list of todo"
    }
    Todo::main(args[1].as_str(), &args[2..]);
}

struct Todo {
    name: String,
    owner: String,
    comment: String,
    done: bool,
    wait: bool,
    dependencies: Vec<String>,
    children: Vec<Rc<RefCell<Todo>>>,
}

impl Todo {
    fn main(mdfile: &str, targets: &[String]) {
        let map = Todo::readmd(mdfile, targets);
        let mut root = map.get(ROOT).unwrap().borrow_mut();

        let mut path: HashSet<String> = HashSet::new();
        let mut visited: HashSet<String> = HashSet::new();
        let screen_width: usize = match termsize::get() {
            None => 80,
            Some(x) => x.cols.into(),
        };
        let mut column_width: [usize; 3] = [0; 3];
        root.build_tree(&map, &mut path, &mut visited, &mut column_width, 0);

        assert!(
            screen_width > column_width[0] + column_width[1] + 8,
            "ERR-002: Screen is too narrow for this todotree markdown file"
        );
        column_width[2] = cmp::min(
            column_width[2],
            screen_width - column_width[0] - column_width[1] - 8,
        );
        Todo::print_header(&column_width);
        let mut connectors: Vec<bool> = Vec::new();
        root.print_tree(&mut connectors, &column_width);
    }

    fn readmd(mdfile: &str, params: &[String]) -> HashMap<String, Rc<RefCell<Todo>>> {
        let mut map: HashMap<String, Rc<RefCell<Todo>>> = HashMap::new();
        let mut name = String::new();
        let mut owner = String::new();
        let mut comment = String::new();
        let mut dependencies: Vec<String> = Vec::new();
		let mut targets = params.to_vec();
/*
        match read_to_string(mdfile) {
			None => panic!("ERR-888: no such a todotree markdown file '{}'", mdfile),
			Some(md) => {
			}
		}
*/
        for ln in read_to_string(mdfile).unwrap().lines() {
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
                owner: "OWNER".to_owned(),
                comment: "COMMENT".to_owned(),
                done: false,
                wait: false,
                dependencies: targets.to_vec(),
                children: Vec::new(),
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
        };
        map.insert(name.clone(), Rc::new(RefCell::new(todo)));
    }

    fn build_tree(
        &mut self,
        map: &HashMap<String, Rc<RefCell<Todo>>>,
        path: &mut HashSet<String>,
        visited: &mut HashSet<String>,
        column_width: &mut [usize; 3],
        depth: usize,
    ) {
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
                        child
                            .borrow_mut()
                            .build_tree(map, path, visited, column_width, depth + 1);
                    }
                    self.wait = self.wait || !child.borrow().done;
                }
            };
            path.remove(dep);
        }
        column_width[0] = cmp::max(column_width[0], depth * 4 + self.name.len());
        column_width[1] = cmp::max(column_width[1], self.owner.len());
        column_width[2] = cmp::max(column_width[2], self.comment.len());
    }

    fn print_tree(&self, connectors: &mut Vec<bool>, column_width: &[usize; 3]) {
        for (pos, cn) in connectors.iter().enumerate() {
            if *cn {
                if pos + 1 < connectors.len() {
                    print!("    ");
                } else {
                    print!("└── ");
                }
            } else if pos + 1 < connectors.len() {
                print!("│   ");
            } else {
                print!("├── ");
            }
        }
        if self.done {
            print!("{}", self.name.strikethrough());
        } else if self.wait {
            print!("{}", self.name);
        } else {
            print!("{}", self.name.red());
        }
        print!(
            "{}",
            " ".repeat(column_width[0] - connectors.len() * 4 - self.name.len())
        );
        print!(" │ ");
        print!("{}", self.owner);
        print!("{}", " ".repeat(column_width[1] - self.owner.len()));
        print!(" │ ");
        self.print_comment(connectors, column_width, &mut 0);
        for (pos, child) in self.children.iter().enumerate() {
            connectors.push(pos + 1 == self.children.len());
            child.borrow().print_tree(connectors, column_width);
            connectors.pop();
        }
    }

    fn print_header(column_width: &[usize; 3]) {
        print!("{}", " ".repeat(column_width[0]));
        print!(" ┌─");
        print!("{}", "─".repeat(column_width[1]));
        print!("─┬─");
        print!("{}", "─".repeat(column_width[2]));
        println!("─┐");
    }

    fn print_comment(&self, connectors: &Vec<bool>, column_width: &[usize; 3], start: &mut usize) {
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
            let slen = cmp::min(self.comment.len() - *start, column_width[2]);
            print!("{}", &self.comment[*start..*start + slen]);
            print!("{}", " ".repeat(column_width[2] - slen));
            println!(" │");
            *start = *start + slen;
            for b in connectors {
                match *b {
                    true => print!(" "),
                    false => print!("│"),
                }
                print!("   ");
            }
            match self.children.len() {
                0 => print!("    "),
                _ => print!("│   "),
            };
            print!("{}", " ".repeat(column_width[0] - 4 - connectors.len() * 4));
            if *start < self.comment.len() {
                print!(" │ ");
                print!("{}", " ".repeat(column_width[1]));
                print!(" │ ");
            } else {
                match last {
                    false => print!(" ├─"),
                    true => print!(" └─"),
                };
                print!("{}", "─".repeat(column_width[1]));
                match last {
                    false => print!("─┼─"),
                    true => print!("─┴─"),
                };
                print!("{}", "─".repeat(column_width[2]));
                match last {
                    false => print!("─┤"),
                    true => print!("─┘"),
                };
                break;
            }
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main() {
        Todo::main("todo.md", &mut Vec::<String>::new());
    }

    #[test]
    #[should_panic]
    fn test_loop1() {
        Todo::main("src/tests/loop1.md", &mut Vec::<String>::new());
    }

    #[test]
    #[should_panic]
    fn test_loop2() {
        Todo::main("src/tests/loop2.md", &mut Vec::<String>::new());
    }
}
