use super::{Format, ROOT, Status, TodoError, todo::Todo};
use libc::{STDOUT_FILENO, TIOCGWINSZ, ioctl, winsize};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::read_to_string;
use std::rc::Rc;

/// A tree of todos
pub struct Tree {
    /// tree root
    root: Rc<RefCell<Todo>>,
    /// output format
    format: Format,
    /// maximum length of the three columns
    maxlens: [usize; 3],
    /// a map of todos, key is map name
    dict: BTreeMap<String, Rc<RefCell<Todo>>>,
    /// a separator joining multiple lines of comments
    separator: String,
    /// auxilary lines before the first todo
    auxilaries: Vec<String>,
    /// no color
    no_color: bool,
    /// reverse
    reverse: bool,
}

impl fmt::Display for Tree {
    fn fmt(&self, fo: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.format == Format::Md {
            for ln in &self.auxilaries {
                writeln!(fo, "{}", ln)?;
            }
        }
        let mut connectors: Vec<bool> = Vec::new();
        let mut visited: BTreeSet<String> = BTreeSet::new();
        self.root.borrow().fmt_tree(
            fo,
            &mut connectors,
            &mut visited,
            &self.maxlens,
            &self.format,
            self.no_color,
            self.reverse,
        )?;
        Ok(())
    }
}

impl Tree {
    /// Creates a tree from a markdown file.
    pub fn new(
        inputs: &Vec<String>,
        owners: &mut BTreeMap<String, bool>,
        targets: &[String],
        term_width: usize,
        format: &str,
        hide_done: bool,
        dpth_limit: i32,
        separator: &str,
        no_color: bool,
        auto_add: bool,
        hide_comment: bool,
        hide_owner: bool,
        reverse: bool,
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
        if reverse && format_enum != Format::Term && format_enum != Format::Html
        {
            return Err(TodoError {
                msg: String::from(
                    "ERR-024: --reverse works with Term or Html only",
                ),
            });
        }
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
            dict: BTreeMap::new(),
            separator: String::from(separator),
            auxilaries: Vec::new(),
            no_color: no_color,
            reverse: reverse,
        };
        for mdfile in inputs {
            tree.readmd(mdfile, hide_comment, hide_owner)?;
        }
        // check dict
        if tree.dict.len() == 0 {
            return Err(TodoError {
                msg: String::from(
                    "ERR-010: The markdown file does not have any Todo",
                ),
            });
        }
        // add all TODOs that have no parent to ROOT's dependencies
        if tree.root.borrow().dependencies.len() == 0 {
            let mut noparent: BTreeSet<String> =
                tree.dict.keys().cloned().collect();
            for todo in tree.dict.values() {
                for dep in &todo.borrow().dependencies {
                    let dep = dep.replace("~", "");
                    noparent.remove(&dep);
                }
                if noparent.len() == 0 {
                    return Err(TodoError {
                        msg: String::from(
                            "ERR-007: Failed to find root node, as all todos \
                             are in dependency loops",
                        ),
                    });
                }
            }
            for nm in tree.dict.keys() {
                if noparent.contains(nm) {
                    tree.root.borrow_mut().dependencies.push(nm.clone());
                }
            }
        }
        tree.check_todos_in_dep_only(auto_add)?;
        let mut path: BTreeSet<String> = BTreeSet::new();
        let mut visited: BTreeSet<String> = BTreeSet::new();
        tree.root.borrow_mut().build_tree(
            &mut visited,
            &tree.dict,
            &mut tree.maxlens,
            &mut path,
            0,
            screen_width,
            hide_done,
            dpth_limit,
            &format_enum,
            owners,
        )?;
        for (owner, used) in owners.iter() {
            if !*used {
                return Err(TodoError {
                    msg: format!(
                        "ERR-022: no such owner '{}' in the markdown file",
                        owner
                    ),
                });
            }
        }
        Ok(tree)
    }

    /// escape markdown string
    fn escape(&mut self, input: &str) -> String {
        static SPECIALS: [char; 15] = [
            '\\', '`', '*', '_', '{', '}', '[', ']', '(', ')', '#', '+', '-',
            '.', '!',
        ];
        let mut escaped = String::from("");
        let mut prev_is_slash = false;
        for mut c in input.chars() {
            if c == '\t' {
                c = ' ';
            }
            if prev_is_slash {
                if !SPECIALS.contains(&c) {
                    escaped.push('\\');
                }
                escaped.push(c);
                prev_is_slash = false;
            } else if c == '\\' {
                prev_is_slash = true;
            } else {
                escaped.push(c);
            }
        }
        if prev_is_slash {
            escaped.push('\\');
        }
        escaped
    }

    /// Creates a list of todos from a markdown fie.
    fn readmd(
        &mut self,
        mdfile: &str,
        hide_comment: bool,
        hide_owner: bool,
    ) -> Result<(), TodoError> {
        let mut name = String::new();
        let mut owner = String::new();
        let mut comment: Vec<String> = Vec::new();
        let mut dependencies: Vec<String> = Vec::new();
        let mut auxilaries: Vec<String> = Vec::new();
        let buffer = match read_to_string(mdfile) {
            Ok(md) => self.escape(&md),
            Err(e) => {
                return Err(TodoError {
                    msg: format!("ERR-008: '{}', {}.", mdfile, e),
                });
            }
        };
        for ln in buffer.lines() {
            let ln = ln.trim();
            if ln.starts_with("# ") {
                self.new_todo_if_any(
                    name,
                    owner,
                    comment,
                    dependencies,
                    auxilaries,
                )?;
                name = match ln.get(2..) {
                    Some(x) => x.trim().to_string(),
                    _ => {
                        return Err(TodoError {
                            msg: format!("ERR-015: '{}'", ln),
                        });
                    }
                };
                if name == "" || name == ROOT {
                    return Err(TodoError {
                        msg: format!(
                            "ERR-009: '{}' is a reserved Todo name keyword",
                            ROOT
                        ),
                    });
                }
                owner = String::new();
                comment = Vec::new();
                dependencies = Vec::new();
                auxilaries = Vec::new();
            } else if ln.starts_with("- @") {
                if !hide_owner {
                    if owner.len() > 0 {
                        owner.push_str(" ");
                    }
                    owner.push_str(ln.get(3..).unwrap().trim())
                }
            } else if ln.starts_with("- %") {
                if !hide_comment {
                    comment.push(ln.get(3..).unwrap().trim().to_string());
                }
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
                for dep in dependencies.iter() {
                    let dep = dep.replace("~", "");
                    if dep == name {
                        return Err(TodoError {
                            msg: format!(
                                "ERR-016: Todo '{}' should not depend on itself",
                                dep
                            ),
                        });
                    }
                }
            } else {
                auxilaries.push(String::from(ln));
            }
        }
        self.new_todo_if_any(name, owner, comment, dependencies, auxilaries)
    }

    /// Returns the todos that are defined in dependencies only.
    fn check_todos_in_dep_only(
        &mut self,
        auto_add: bool,
    ) -> Result<(), TodoError> {
        let mut noparent: BTreeSet<String> =
            self.dict.keys().cloned().collect();
        let mut todoindepsonly: BTreeMap<String, (String, Todo)> =
            BTreeMap::new();
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
                        if auto_add {
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
        }
        for (k, v) in todoindepsonly {
            self.dict.insert(k, Rc::new(RefCell::new(v.1)));
        }
        Ok(())
    }

    /// Creates todo
    ///
    /// When readmd reads the second todo or reaches the markdown file end.
    fn new_todo_if_any(
        &mut self,
        name: String,
        owner: String,
        comment: Vec<String>,
        dependencies: Vec<String>,
        auxilaries: Vec<String>,
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
        if self
            .dict
            .insert(nm.clone(), Rc::new(RefCell::new(todo)))
            .is_some()
        {
            return Err(TodoError {
                msg: format!("ERR-014: Duplicated todo name '{}'", nm),
            });
        }
        Ok(())
    }
}
