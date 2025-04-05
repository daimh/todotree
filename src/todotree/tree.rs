use super::{Format, HTMLP, ROOT, Status, TodoError, todo::Todo};
use libc::{STDOUT_FILENO, TIOCGWINSZ, ioctl, winsize};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs::read_to_string;
use std::rc::Rc;

pub struct Tree {
    root: Rc<RefCell<Todo>>,
    format: Format,
    maxlens: [usize; 3],
    dict: HashMap<String, Rc<RefCell<Todo>>>,
    separator: String,
    auxilaries: Vec<String>,
}

impl fmt::Display for Tree {
    fn fmt(&self, fo: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_header(fo)?;
        let mut connectors: Vec<bool> = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        self.root.borrow().fmt_tree(
            fo,
            &mut connectors,
            &mut visited,
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
        term_width: usize,
        format: &str,
        hide: bool,
        dpth_limit: i32,
        separator: &str,
    ) -> Result<Self, TodoError> {
        let format_enum = match format {
            "html" => Format::Html,
            "json" => Format::Json,
            "term" => Format::Term,
            "md" => Format::Md,
            "" => Format::Term,
            _ => {
                return Err(TodoError {
                    msg: String::from("ERR-006: Wrong parameter for -f"),
                });
            }
        };
        let mut screen_width: usize = 80;
        if format_enum == Format::Term && term_width == 0 {
            let mut ws = winsize {
                ws_row: 0,
                ws_col: 0,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            unsafe {
                if ioctl(STDOUT_FILENO, TIOCGWINSZ, &mut ws) != -1
                    && ws.ws_col > 0
                {
                    screen_width = ws.ws_col as usize;
                }
            }
        };
        let mut tree = Tree {
            root: Rc::new(RefCell::new(Todo::new(
                String::from(ROOT),
                String::new(),
                Vec::new(),
                targets.to_vec(),
                Vec::new(),
            )?)),
            format: format_enum.clone(),
            maxlens: [0; 3],
            dict: HashMap::new(),
            separator: String::from(separator),
            auxilaries: Vec::new(),
        };
        let todolist = tree.readmd(mdfile)?;
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
            for nm in todolist {
                if noparent.contains(&nm) {
                    tree.root.borrow_mut().dependencies.push(nm.clone());
                }
            }
        }
        tree.get_todos_in_dep_only()?;
        let mut path: HashSet<String> = HashSet::new();
        let mut visited: HashSet<String> = HashSet::new();
        tree.root.borrow_mut().build_tree(
            &mut visited,
            &tree.dict,
            &mut tree.maxlens,
            &mut path,
            0,
            screen_width,
            hide,
            dpth_limit,
            &format_enum,
        )?;
        Ok(tree)
    }

    fn readmd(&mut self, mdfile: &str) -> Result<Vec<String>, TodoError> {
        let mut name = String::new();
        let mut owner = String::new();
        let mut comment: Vec<String> = Vec::new();
        let mut dependencies: Vec<String> = Vec::new();
        let mut auxilaries: Vec<String> = Vec::new();
        let buffer = match read_to_string(mdfile) {
            Ok(md) => md,
            Err(e) => {
                return Err(TodoError {
                    msg: format!("ERR-008: '{}', {}.", mdfile, e),
                });
            }
        };
        let mut todolist: Vec<String> = Vec::new();
        for mut ln in buffer.lines() {
            ln = ln.trim();
            if ln.starts_with("# ") {
                self.new_todo_if_any(
                    name,
                    owner,
                    comment,
                    dependencies,
                    auxilaries,
                    &mut todolist,
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
                comment = Vec::new();
                dependencies = Vec::new();
                auxilaries = Vec::new();
            } else if ln.starts_with("- @") {
                if owner.len() > 0 {
                    owner.push_str(" ");
                }
                owner.push_str(ln.get(3..).unwrap().trim())
            } else if ln.starts_with("- %") {
                comment.push(ln.get(3..).unwrap().trim().to_string());
            } else if ln.starts_with("- :") {
                dependencies.append(
                    &mut ln
                        .get(3..)
                        .unwrap()
                        .trim()
                        .split_whitespace()
                        .map(str::to_string)
                        .collect::<Vec<String>>(),
                );
            } else {
                auxilaries.push(String::from(ln));
            }
        }
        self.new_todo_if_any(
            name,
            owner,
            comment,
            dependencies,
            auxilaries,
            &mut todolist,
        )?;
        match self.dict.len() {
            0 => Err(TodoError {
                msg: String::from(
                    "ERR-010: The markdown file doesn't have any Todo.",
                ),
            }),
            _ => Ok(todolist),
        }
    }

    fn get_todos_in_dep_only(&mut self) -> Result<(), TodoError> {
        let mut noparent: HashSet<&String> =
            HashSet::from_iter(self.dict.keys());
        let mut todoindepsonly: HashMap<String, (String, Todo)> =
            HashMap::new();
        for (key, todo) in &self.dict {
            for dep_raw in &todo.borrow().dependencies {
                let dep_nom = String::from(dep_raw.replace("~", "").trim());
                noparent.remove(&dep_nom);
                let cur_completed = dep_raw.contains("~");
                if self.dict.contains_key(&dep_nom) {
                    if cur_completed {
                        return Err(TodoError {
                            msg: format!(
                                "ERR-011: Todo '{}' has its own '# ' line, \
									then it should not have '~' in '{}'s \
									dependencies list.",
                                dep_nom, key
                            ),
                        });
                    }
                    continue;
                }
                match todoindepsonly.get(&dep_nom) {
                    Some(parent_todo) => {
                        let prv_completed =
                            parent_todo.1.status == Status::Completed;
                        if prv_completed != cur_completed {
                            return Err(TodoError {
                                msg: format!(
                                    "ERR-012: Todo '{}' has a dependency \
									'~{}', but todo '{}' has a dependency \
									'{}'.",
                                    key, dep_nom, parent_todo.0, dep_nom
                                ),
                            });
                        }
                    }
                    None => {
                        todoindepsonly.insert(
                            dep_nom.clone(),
                            (
                                String::from(key),
                                Todo::new(
                                    dep_raw.clone(),
                                    String::new(),
                                    Vec::new(),
                                    Vec::new(),
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
        comment: Vec<String>,
        dependencies: Vec<String>,
        auxilaries: Vec<String>,
        todolist: &mut Vec<String>,
    ) -> Result<(), TodoError> {
        if name == "" {
            self.auxilaries = auxilaries;
            if owner == "" && comment.len() == 0 && dependencies.len() == 0 {
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
        let comt: Vec<String> = match self.separator.as_str() {
            "\n" => comment,
            _ => {
                vec![comment.join(self.separator.as_str()); 1]
            }
        };
        let todo = Todo::new(name, owner, comt, dependencies, auxilaries)?;
        let nm = todo.name.clone();
        todolist.push(nm.clone());
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
            Format::Md => {
                for ln in &self.auxilaries {
                    writeln!(fo, "{}", ln)?;
                }
            }
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
