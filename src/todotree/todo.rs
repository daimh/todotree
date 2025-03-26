use super::Format;
use super::HTMLP;
use super::ROOT;
use super::Status;
use super::TodoError;
use std::cell::RefCell;
use std::cmp::{max, min};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::rc::Rc;

pub struct Todo {
    pub name: String,
    owner: String,
    comment: String,
    pub status: Status,
    pub dependencies: Vec<String>,
    children: Vec<Rc<RefCell<Todo>>>,
    height: i32,
}

impl Todo {
    pub fn new(
        name: String,
        owner: String,
        comment: String,
        dependencies: Vec<String>,
    ) -> Result<Self, TodoError> {
        let status = match name.starts_with("~") {
            true => Status::Completed,
            false => Status::Pending,
        };
        let realname = match status {
            Status::Completed => String::from(name.replace("~", "").trim()),
            _ => name.clone(),
        };
        static SPECIALS: [char; 17] = [
            '!', '@', '$', '%', '%', '&', '(', ')', '-', '_', '=', '+', ':',
            '\'', '"', '.', '?',
        ];
        if realname != ROOT {
            for c in realname.chars() {
                if !SPECIALS.contains(&c)
                    && (c < 'a' || c > 'z')
                    && (c < 'A' || c > 'Z')
                    && (c < '0' || c > '9')
                {
                    return Err(TodoError {
                        msg: format!(
                            "ERR-001: Todo name '{}' contains some character \
						'{}', which is not alphabet, digit, or {:?}.",
                            name, c, SPECIALS
                        ),
                    });
                }
            }
        }
        Ok(Todo {
            name: realname,
            owner: owner,
            comment: comment,
            status: status,
            dependencies: dependencies,
            children: Vec::new(),
            height: 0,
        })
    }

    pub fn build_tree(
        &mut self,
        map: &HashMap<String, Rc<RefCell<Todo>>>,
        maxlens: &mut [usize; 3],
        path: &mut HashSet<String>,
        visited: &mut HashSet<String>,
        depth: usize,
        screen_width: usize,
        hide: bool,
        dpth_limit: i32,
    ) -> Result<(), TodoError> {
        let mut notdonedeps: Vec<String> = vec![];
        for dep_raw in &self.dependencies {
            let dep_nm = String::from(dep_raw.replace("~", "").trim());
            if !path.insert(dep_nm.clone()) {
                return Err(TodoError {
                    msg: format!(
                        "ERR-002: Todo '{}' has a dependency loop.",
                        self.name
                    ),
                });
            }
            let child = match map.get(&dep_nm) {
                Some(x) => x,
                _ => panic!("ERR-003: {} is missing", &dep_nm),
            };
            let dep_notdone = child.borrow().status != Status::Completed;
            if dep_notdone {
                notdonedeps.push(dep_nm.clone());
            }
            if (dpth_limit <= 0 || dpth_limit > depth as i32)
                && visited.insert(dep_nm.clone())
            {
                child.borrow_mut().build_tree(
                    map,
                    maxlens,
                    path,
                    visited,
                    depth + 1,
                    screen_width,
                    hide,
                    dpth_limit,
                )?;
                let child_height = child.borrow().height;
                self.height = max(child_height + 1, self.height);
                if (dpth_limit >= 0 || child_height + dpth_limit >= 0)
                    && (dep_notdone || !hide)
                {
                    self.children.push(Rc::clone(child));
                }
            }
            path.remove(&dep_nm.clone());
        }
        if notdonedeps.len() == 0 {
            if self.status != Status::Completed {
                self.status = Status::Actionable;
            }
        } else if self.status == Status::Completed {
            return Err(TodoError {
                msg: format!(
                    "ERR-004: Todo \"{}\" cannot be marked as completed \
				because its dependencies {:?} are yet completed.",
                    self.name, notdonedeps
                ),
            });
        }
        if self.dependencies.len() > 0
            && ((dpth_limit > 0 && dpth_limit == depth as i32)
                || (dpth_limit < 0 && self.height + dpth_limit == 0))
        {
            self.name.push_str("/")
        }
        if self.name == ROOT {
            if maxlens[1] > 0 {
                self.owner = String::from("OWNER")
            }
            if maxlens[2] > 0 {
                self.comment = String::from("COMMENT")
            }
            if screen_width <= maxlens[0] + maxlens[1] + 8 {
                return Err(TodoError {
                    msg: String::from(
                        "ERR-005: Screen is too narrow for this todotree \
						markdown file.",
                    ),
                });
            }
            maxlens[2] =
                min(maxlens[2], screen_width - maxlens[0] - maxlens[1] - 8);
        }
        maxlens[0] = max(maxlens[0], depth * 4 + self.name.len());
        maxlens[1] = max(maxlens[1], self.owner.len());
        maxlens[2] = max(maxlens[2], self.comment.len());
        Ok(())
    }

    pub fn fmt_tree(
        &self,
        fo: &mut fmt::Formatter<'_>,
        connectors: &mut Vec<bool>,
        maxlens: &[usize; 3],
        format: &Format,
    ) -> fmt::Result {
        let space: String;
        match format {
            Format::Json => {
                space = " ".repeat(connectors.len() * 4);
                writeln!(fo, "{}{{", space)?;
                writeln!(fo, "{}  \"name\": \"{}\",", space, self.name)?;
                writeln!(fo, "{}  \"status\": \"{}\",", space, self.status)?;
                if maxlens[1] > 0 {
                    writeln!(fo, "{}  \"owner\": \"{}\",", space, self.owner)?;
                }
                if maxlens[2] > 0 {
                    writeln!(
                        fo,
                        "{}  \"comment\": \"{}\",",
                        space, self.comment
                    )?;
                }
                writeln!(fo, "{}  \"dependencies\": [", space)?;
            }
            Format::Term => {
                space = String::from(" ");
                self.fmt_connector(fo, connectors, &space)?;
                match self.status {
                    Status::Completed => write!(fo, "\x1b\x5b\x33\x34\x6d")?,
                    Status::Actionable => write!(fo, "\x1b\x5b\x33\x31\x6d")?,
                    Status::Pending => (),
                }
                write!(fo, "{}", self.name)?;
                if self.status != Status::Pending {
                    write!(fo, "\x1b\x28\x42\x1b\x5b\x6d")?;
                }
                self.fmt_table(fo, connectors, &space, maxlens, format)?;
            }
            Format::Html => {
                space = String::from("&nbsp;");
                write!(fo, "{}", HTMLP)?;
                self.fmt_connector(fo, connectors, &space)?;
                match self.status {
                    Status::Completed => {
                        write!(fo, "<span style='color:blue'>")?
                    }
                    Status::Actionable => {
                        write!(fo, "<span style='color:red'>")?
                    }
                    Status::Pending => (),
                }
                write!(fo, "{}", self.name)?;
                if self.status != Status::Pending {
                    write!(fo, "</span>")?;
                }
                self.fmt_table(fo, connectors, &space, maxlens, format)?;
            }
        }
        for (pos, child) in self.children.iter().enumerate() {
            connectors.push(pos + 1 == self.children.len());
            if pos > 0 && *format == Format::Json {
                writeln!(fo, "{}    ,", space)?;
            }
            child.borrow().fmt_tree(fo, connectors, maxlens, format)?;
            connectors.pop();
        }
        if *format == Format::Json {
            writeln!(fo, "{}  ]", space)?;
            writeln!(fo, "{}}}", space)?;
        }
        Ok(())
    }

    fn fmt_connector(
        &self,
        fo: &mut fmt::Formatter<'_>,
        connectors: &mut Vec<bool>,
        space: &String,
    ) -> fmt::Result {
        for (pos, cn) in connectors.iter().enumerate() {
            if *cn {
                if pos + 1 < connectors.len() {
                    write!(fo, "{}", space.repeat(4))?;
                } else {
                    write!(fo, "└──{}", space)?;
                }
            } else if pos + 1 < connectors.len() {
                write!(fo, "│{}", space.repeat(3))?;
            } else {
                write!(fo, "├──{}", space)?;
            }
        }
        Ok(())
    }

    fn fmt_table(
        &self,
        fo: &mut fmt::Formatter<'_>,
        connectors: &mut Vec<bool>,
        space: &String,
        maxlens: &[usize; 3],
        format: &Format,
    ) -> fmt::Result {
        match maxlens[1] + maxlens[2] {
            0 => writeln!(fo),
            _ => {
                write!(
                    fo,
                    "{}",
                    space.repeat(
                        maxlens[0] - connectors.len() * 4 - self.name.len()
                    )
                )?;
                write!(fo, "{}│{}", space, space)?;
                write!(fo, "{}", self.owner)?;
                write!(fo, "{}", space.repeat(maxlens[1] - self.owner.len()))?;
                if maxlens[1] > 0 && maxlens[2] > 0 {
                    write!(fo, "{}│{}", space, space)?;
                }
                self.fmt_comment(fo, connectors, &mut 0, maxlens, format)
            }
        }
    }

    fn fmt_comment(
        &self,
        fo: &mut fmt::Formatter<'_>,
        connectors: &Vec<bool>,
        start: &mut usize,
        maxlens: &[usize; 3],
        format: &Format,
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
        let (space, eol) = match format {
            Format::Html => (String::from("&nbsp;"), format!("</p>\n")),
            _ => (String::from(" "), String::from("\n")),
        };
        loop {
            let slen = min(self.comment.len() - *start, maxlens[2]);
            write!(fo, "{}", &self.comment[*start..*start + slen])?;
            write!(fo, "{}", space.repeat(maxlens[2] - slen))?;
            write!(fo, "{}│{}", space, eol)?;
            if *format == Format::Html {
                write!(fo, "{}", HTMLP)?;
            }
            *start = *start + slen;
            for b in connectors {
                match *b {
                    true => write!(fo, "{}", space)?,
                    false => write!(fo, "│")?,
                };
                write!(fo, "{}", space.repeat(3))?;
            }
            match self.children.len() {
                0 => write!(fo, "{}", space),
                _ => write!(fo, "│"),
            }?;
            write!(
                fo,
                "{}",
                space.repeat(maxlens[0] - 1 - connectors.len() * 4)
            )?;
            if *start < self.comment.len() {
                write!(fo, "{}│{}", space, space)?;
                write!(fo, "{}", space.repeat(maxlens[1]))?;
                if maxlens[1] > 0 && maxlens[2] > 0 {
                    write!(fo, "{}│{}", space, space)?;
                }
            } else {
                match last {
                    false => write!(fo, "{}├─", space),
                    true => write!(fo, "{}└─", space),
                }?;
                write!(fo, "{}", "─".repeat(maxlens[1]))?;
                if maxlens[1] > 0 && maxlens[2] > 0 {
                    match last {
                        false => write!(fo, "─┼─"),
                        true => write!(fo, "─┴─"),
                    }?;
                }
                write!(fo, "{}", "─".repeat(maxlens[2]))?;
                match last {
                    false => write!(fo, "─┤"),
                    true => write!(fo, "─┘"),
                }?;
                write!(fo, "{}", eol)?;
                break;
            }
        }
        Ok(())
    }
}
