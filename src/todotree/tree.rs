use crate::todotree::HTMLP;
use crate::todotree::ROOT;
use crate::todotree::todo::Status;
use crate::todotree::todo::Todo;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::fs::read_to_string;
use std::rc::Rc;

#[derive(PartialEq)]
pub enum Format {
    Html,
    Json,
    Term,
}

pub struct Tree {
    root: Rc<RefCell<Todo>>,
    format: Format,
    maxlens: [usize; 3],
    map: HashMap<String, Rc<RefCell<Todo>>>,
}

impl fmt::Display for Tree {
    fn fmt(&self, fo: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_header(fo)?;
        let mut connectors: Vec<bool> = Vec::new();
        self.root.borrow().fmt_tree(
            fo,
            &mut connectors,
            &self.maxlens,
            &self.format,
        )?;
        Ok(())
    }
}

impl Tree {
    pub fn new(
        mdfile: &str,
        targets: &[String],
        default_screen_width: usize,
        format: &str,
        hide: bool,
    ) -> Self {
        let format_enum = match format {
            "html" => Format::Html,
            "json" => Format::Json,
            "term" => Format::Term,
            "" => Format::Term,
            _ => panic!("ERR-013: Wrong format string."),
        };
        let screen_width: usize = match format_enum {
            Format::Term => match default_screen_width {
                0 => match termsize::get() {
                    None => 80,
                    Some(x) => x.cols.into(),
                },
                _ => default_screen_width,
            },
            _ => 80,
        };
        let mut tree = Tree {
            root: Rc::new(RefCell::new(Todo::new(
                String::from(ROOT),
                String::new(),
                String::new(),
                targets.to_vec(),
            ))),
            format: format_enum,
            maxlens: [0; 3],
            map: HashMap::new(),
        };
        let list = tree.readmd(mdfile);
        if tree.root.borrow().dependencies.len() == 0 {
            let mut noparent: HashSet<&String> =
                HashSet::from_iter(tree.map.keys());
            for todo in tree.map.values() {
                for dep_raw in &todo.borrow().dependencies {
                    let dep_nm = String::from(dep_raw.replace("~", "").trim());
                    noparent.remove(&dep_nm);
                }
                assert!(
                    noparent.len() > 0,
                    "ERR-013: All todos are in a dependency loop."
                );
            }
            for nm in list {
                if noparent.contains(&nm) {
                    tree.root.borrow_mut().dependencies.push(nm.clone());
                }
            }
        }
        tree.get_todos_in_dep_only();
        let mut path: HashSet<String> = HashSet::new();
        let mut visited: HashSet<String> = HashSet::new();
        tree.root.borrow_mut().build_tree(
            &tree.map,
            &mut tree.maxlens,
            &mut path,
            &mut visited,
            0,
            screen_width,
            hide,
        );
        tree
    }

    fn readmd(&mut self, mdfile: &str) -> Vec<String> {
        let mut name = String::new();
        let mut owner = String::new();
        let mut comment = String::new();
        let mut dependencies: Vec<String> = Vec::new();
        let buffer = match read_to_string(mdfile) {
            Ok(md) => md,
            Err(e) => panic!("ERR-008: '{}', {}.", mdfile, e),
        };
        let mut list: Vec<String> = Vec::new();
        for ln in buffer.lines() {
            if ln.starts_with("# ") {
                self.new_todo_if_any(
                    name,
                    owner,
                    comment,
                    dependencies,
                    &mut list,
                );
                name = ln.get(2..).unwrap().trim().to_string();
                assert!(
                    name != "" && name != ROOT,
                    "ERR-009: '{}' is a reserved Todo name keyword.",
                    ROOT
                );
                owner = String::new();
                comment = String::new();
                dependencies = Vec::new();
            } else if ln.starts_with("- @ ") {
                owner = ln.get(4..).unwrap().trim().to_string();
            } else if ln.starts_with("- % ") {
                comment = ln.get(4..).unwrap().trim().to_string();
            } else if ln.starts_with("- : ") {
                dependencies.append(
                    &mut ln
                        .get(4..)
                        .unwrap()
                        .split_whitespace()
                        .map(str::to_string)
                        .collect::<Vec<String>>(),
                );
            }
        }
        self.new_todo_if_any(name, owner, comment, dependencies, &mut list);
        assert!(
            self.map.len() > 0,
            "ERR-004: The markdown file doesn't have any Todo."
        );
        list
    }

    fn get_todos_in_dep_only(&mut self) {
        let mut noparent: HashSet<&String> =
            HashSet::from_iter(self.map.keys());
        let mut todoindepsonly: HashMap<String, (String, Todo)> =
            HashMap::new();
        for (key, todo) in &self.map {
            for dep_raw in &todo.borrow().dependencies {
                let dep_nm = String::from(dep_raw.replace("~", "").trim());
                noparent.remove(&dep_nm);
                let cur_completed = dep_raw.contains("~");
                if self.map.contains_key(&dep_nm) {
                    if cur_completed {
                        panic!(
                            "ERR-018: Todo '{}' has its own '# ' line, \
									then it should not have '~' in '{}'s \
									dependencies list.",
                            dep_nm, key
                        );
                    }
                    continue;
                }
                match todoindepsonly.get(&dep_nm) {
                    Some(parent_todo) => {
                        let prv_completed =
                            parent_todo.1.status == Status::Completed;
                        if prv_completed != cur_completed {
                            panic!(
                                "ERR-019: Todo '{}' has a dependency '~{}', but \
									todo '{}' has a dependency '{}'.",
                                key, dep_nm, parent_todo.0, dep_nm
                            );
                        }
                    }
                    None => {
                        todoindepsonly.insert(
                            dep_nm.clone(),
                            (
                                String::from(key),
                                Todo::new(
                                    dep_raw.clone(),
                                    String::new(),
                                    String::new(),
                                    Vec::new(),
                                ),
                            ),
                        );
                    }
                }
            }
        }
        for (k, v) in todoindepsonly {
            self.map.insert(k, Rc::new(RefCell::new(v.1)));
        }
    }

    fn new_todo_if_any(
        &mut self,
        name: String,
        owner: String,
        comment: String,
        dependencies: Vec<String>,
        list: &mut Vec<String>,
    ) {
        if name == "" {
            assert!(
                owner == "" && comment == "" && dependencies.len() == 0,
                "ERR-006: Missing '# [TODO]' before '- @', '- :', or '-  %' \
				in the todotree markdown file."
            );
            return;
        }
        let todo = Todo::new(name, owner, comment, dependencies);
        let nm = todo.name.clone();
        list.push(nm.clone());
        if self
            .map
            .insert(nm.clone(), Rc::new(RefCell::new(todo)))
            .is_some()
        {
            panic!("ERR-016: Duplicated todo name '{}'.", nm)
        }
    }

    fn fmt_header(&self, fo: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.format {
            Format::Json => (),
            _ => {
                let space = match self.format {
                    Format::Html => String::from("&nbsp;"),
                    _ => String::from(" "),
                };
                if self.maxlens[1] + self.maxlens[2] > 0 {
                    if self.format == Format::Html {
                        write!(fo, "{}", HTMLP)?;
                    }
                    write!(fo, "{}", space.repeat(self.maxlens[0] + 1))?;
                    write!(fo, "┌─")?;
                    write!(fo, "{}", "─".repeat(self.maxlens[1]))?;
                    if self.maxlens[1] > 0 && self.maxlens[2] > 0 {
                        write!(fo, "─┬─")?;
                    }
                    write!(fo, "{}", "─".repeat(self.maxlens[2]))?;
                    write!(fo, "─┐")?;
                    if self.format == Format::Html {
                        write!(fo, "</p>")?;
                    }
                    writeln!(fo)?;
                } else {
                    write!(fo, "")?;
                }
            }
        }
        Ok(())
    }
}
