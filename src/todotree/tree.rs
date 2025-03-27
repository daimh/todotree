use super::Format;
use super::HTMLP;
use super::ROOT;
use super::Status;
use super::TodoError;
use super::todo::Todo;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::fs::read_to_string;
use std::rc::Rc;

pub struct Tree {
    root: Rc<RefCell<Todo>>,
    format: Format,
    maxlens: [usize; 3],
    dict: HashMap<String, Rc<RefCell<Todo>>>,
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
        dpth_limit: i32,
    ) -> Result<Self, TodoError> {
        let format_enum = match format {
            "html" => Format::Html,
            "json" => Format::Json,
            "term" => Format::Term,
            "" => Format::Term,
            _ => {
                return Err(TodoError {
                    msg: String::from("ERR-006: Wrong parameter for -f"),
                });
            }
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
            )?)),
            format: format_enum,
            maxlens: [0; 3],
            dict: HashMap::new(),
        };
        let list = tree.readmd(mdfile)?;
        if tree.root.borrow().dependencies.len() == 0 {
            let mut noparent: HashSet<&String> =
                HashSet::from_iter(tree.dict.keys());
            for todo in tree.dict.values() {
                for dep_raw in &todo.borrow().dependencies {
                    let dep_nm = String::from(dep_raw.replace("~", "").trim());
                    noparent.remove(&dep_nm);
                }
                if noparent.len() == 0 {
                    return Err(TodoError {
                        msg: String::from(
                            "ERR-007: All todos are in a dependency loop.",
                        ),
                    });
                }
            }
            for nm in list {
                if noparent.contains(&nm) {
                    tree.root.borrow_mut().dependencies.push(nm.clone());
                }
            }
        }
        tree.get_todos_in_dep_only()?;
        let mut path: HashSet<String> = HashSet::new();
        let mut visited: HashSet<String> = HashSet::new();
        tree.root.borrow_mut().build_tree(
            &tree.dict,
            &mut tree.maxlens,
            &mut path,
            &mut visited,
            0,
            screen_width,
            hide,
            dpth_limit,
        )?;
        Ok(tree)
    }

    fn readmd(&mut self, mdfile: &str) -> Result<Vec<String>, TodoError> {
        let mut name = String::new();
        let mut owner = String::new();
        let mut comment = String::new();
        let mut dependencies: Vec<String> = Vec::new();
        let buffer = match read_to_string(mdfile) {
            Ok(md) => md,
            Err(e) => {
                return Err(TodoError {
                    msg: format!("ERR-008: '{}', {}.", mdfile, e),
                });
            }
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
                )?;
                name = match ln.get(2..) {
                    Some(x) => x.trim().to_string(),
                    _ => {
                        return Err(TodoError {
                            msg: format!("ERR-015: '{}'.", ln),
                        });
                    }
                };
                if name == "" || name == ROOT {
                    return Err(TodoError {
                        msg: format!(
                            "ERR-009: '{}' is a reserved Todo name keyword.",
                            ROOT
                        ),
                    });
                }
                owner = String::new();
                comment = String::new();
                dependencies = Vec::new();
            } else if ln.starts_with("- @ ") {
                self.add_line(&mut owner, ln);
            } else if ln.starts_with("- % ") {
                self.add_line(&mut comment, ln);
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
        self.new_todo_if_any(name, owner, comment, dependencies, &mut list)?;
        match self.dict.len() {
            0 => Err(TodoError {
                msg: String::from(
                    "ERR-010: The markdown file doesn't have any Todo.",
                ),
            }),
            _ => Ok(list),
        }
    }

    fn get_todos_in_dep_only(&mut self) -> Result<(), TodoError> {
        let mut noparent: HashSet<&String> =
            HashSet::from_iter(self.dict.keys());
        let mut todoindepsonly: HashMap<String, (String, Todo)> =
            HashMap::new();
        for (key, todo) in &self.dict {
            for dep_raw in &todo.borrow().dependencies {
                let dep_nm = String::from(dep_raw.replace("~", "").trim());
                noparent.remove(&dep_nm);
                let cur_completed = dep_raw.contains("~");
                if self.dict.contains_key(&dep_nm) {
                    if cur_completed {
                        return Err(TodoError {
                            msg: format!(
                                "ERR-011: Todo '{}' has its own '# ' line, \
									then it should not have '~' in '{}'s \
									dependencies list.",
                                dep_nm, key
                            ),
                        });
                    }
                    continue;
                }
                match todoindepsonly.get(&dep_nm) {
                    Some(parent_todo) => {
                        let prv_completed =
                            parent_todo.1.status == Status::Completed;
                        if prv_completed != cur_completed {
                            return Err(TodoError {
                                msg: format!(
                                    "ERR-012: Todo '{}' has a dependency \
									'~{}', but todo '{}' has a dependency \
									'{}'.",
                                    key, dep_nm, parent_todo.0, dep_nm
                                ),
                            });
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
                                )?,
                            ),
                        );
                    }
                }
            }
        }
        for (k, v) in todoindepsonly {
            self.dict.insert(k, Rc::new(RefCell::new(v.1)));
        }
        Ok(())
    }

    fn new_todo_if_any(
        &mut self,
        name: String,
        owner: String,
        comment: String,
        dependencies: Vec<String>,
        list: &mut Vec<String>,
    ) -> Result<(), TodoError> {
        if name == "" {
            if owner == "" && comment == "" && dependencies.len() == 0 {
                return Ok(());
            } else {
                return Err(TodoError {
                    msg: String::from(
                        "ERR-013: Missing '# [TODO]' before '- @', '- :', \
						or '-  %' in the todotree markdown file.",
                    ),
                });
            }
        }
        let todo = Todo::new(name, owner, comment, dependencies)?;
        let nm = todo.name.clone();
        list.push(nm.clone());
        if self
            .dict
            .insert(nm.clone(), Rc::new(RefCell::new(todo)))
            .is_some()
        {
            return Err(TodoError {
                msg: format!("ERR-014: Duplicated todo name '{}'.", nm),
            });
        }
        Ok(())
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

    fn add_line(&self, merged: &mut String, line: &str) {
        if merged.len() > 0 {
            merged.push_str(" ");
        }
        merged.push_str(line.get(4..).unwrap().trim())
    }
}
