use crate::todotree::HTMLP;
use crate::todotree::ROOT;
use crate::todotree::tree::Format;
use std::cell::RefCell;
use std::cmp::{max, min};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::rc::Rc;

#[derive(PartialEq)]
pub enum Status {
    Completed,
    Waiting,
    Doing,
}

pub struct Todo {
    pub name: String,
    owner: String,
    comment: String,
    status: Status,
    pub dependencies: Vec<String>,
    children: Vec<Rc<RefCell<Todo>>>,
}

impl Todo {
    pub fn new(
        name: String,
        owner: String,
        comment: String,
        dependencies: Vec<String>,
    ) -> Self {
        let status = match name.starts_with("~") {
            true => Status::Completed,
            false => Status::Waiting,
        };
        let realname = match status {
            Status::Completed => String::from(name.replace("~", "").trim()),
            _ => name.clone(),
        };
        static SPECIALS: [char; 18] = [
            '!', '@', '$', '%', '%', '&', '(', ')', '-', '_', '=', '+', ':',
            '\'', '"', '.', '/', '?',
        ];
        for c in realname.chars() {
            assert!(
                SPECIALS.contains(&c)
                    || (c >= 'a' && c <= 'z')
                    || (c >= 'A' && c <= 'Z')
                    || (c >= '0' && c <= '9'),
                "ERR-003: todo name '{}' contains some character '{}', \
				which is not alphabet, digit, or {:?}",
                name,
                c,
                SPECIALS
            );
        }
        Todo {
            name: realname,
            owner: owner,
            comment: comment,
            status: status,
            dependencies: dependencies,
            children: Vec::new(),
        }
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
    ) {
        let mut notdonedeps: Vec<&String> = vec![];
        for dep in &self.dependencies {
            assert!(
                path.insert(dep.to_string()),
                "ERR-007: Todo '{}' has a dependency loop",
                self.name
            );
            let child = match map.get(dep) {
                Some(x) => x,
                None => {
                    let todo = Todo::new(
                        String::from(dep),
                        String::new(),
                        String::new(),
                        Vec::new(),
                    );
                    &Rc::new(RefCell::new(todo))
                }
            };
            let dep_notdone = child.borrow().status != Status::Completed;
            if dep_notdone {
                notdonedeps.push(dep);
            }
            if visited.insert(dep.to_string()) {
                child.borrow_mut().build_tree(
                    map,
                    maxlens,
                    path,
                    visited,
                    depth + 1,
                    screen_width,
                    hide,
                );
                if dep_notdone || !hide {
                    self.children.push(Rc::clone(child));
                }
            }
            path.remove(dep);
        }
        if notdonedeps.len() == 0 {
            if self.status != Status::Completed {
                self.status = Status::Doing;
            }
        } else if self.status == Status::Completed {
            panic!(
                "ERR-017: todo \"{}\" cannot be marked as completed \
				because its dependencies {:?} are yet completed",
                self.name, notdonedeps
            );
        }
        if self.name == ROOT {
            if maxlens[1] > 0 {
                self.owner = String::from("OWNER")
            }
            if maxlens[2] > 0 {
                self.comment = String::from("COMMENT")
            }
            assert!(
                screen_width > maxlens[0] + maxlens[1] + 8,
                "ERR-002: Screen is too narrow for this todotree markdown file"
            );
            maxlens[2] =
                min(maxlens[2], screen_width - maxlens[0] - maxlens[1] - 8);
        }
        maxlens[0] = max(maxlens[0], depth * 4 + self.name.len());
        maxlens[1] = max(maxlens[1], self.owner.len());
        maxlens[2] = max(maxlens[2], self.comment.len());
    }

    pub fn fmt_tree(
        &self,
        fo: &mut fmt::Formatter<'_>,
        connectors: &mut Vec<bool>,
        maxlens: &[usize; 3],
        format: &Format,
    ) -> fmt::Result {
        let space = match format {
            Format::Json => " ".repeat(connectors.len() * 4),
            Format::Html => String::from("&nbsp;"),
            Format::Term => String::from(" "),
        };
        match format {
            Format::Json => {
                writeln!(fo, "{}{{", space)?;
                writeln!(fo, "{}  \"name\": \"{}\",", space, self.name)?;
                write!(fo, "{}  \"status\": ", space)?;
                match self.status {
                    Status::Completed => writeln!(fo, "\"strikethrough\",")?,
                    Status::Waiting => writeln!(fo, "null,",)?,
                    Status::Doing => writeln!(fo, "\"red\",",)?,
                }
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
            _ => {
                if *format == Format::Html {
                    write!(fo, "{}", HTMLP)?;
                }
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
                match self.status {
                    // strikethrough
                    Status::Completed => write!(
                        fo,
                        "{}",
                        match format {
                            Format::Term => "\x1b\x5b\x39\x6d",
                            Format::Html =>
                                "<span style='text-decoration:line-through'>",
                            Format::Json => panic!("ERR-010"),
                        }
                    )?,
                    // red
                    Status::Doing => write!(
                        fo,
                        "{}",
                        match format {
                            Format::Term => "\x1b\x5b\x33\x31\x6d",
                            Format::Html => "<span style='color:red'>",
                            Format::Json => panic!("ERR-011"),
                        }
                    )?,
                    // red
                    Status::Waiting => (),
                }
                write!(fo, "{}", self.name)?;
                if self.status != Status::Waiting {
                    write!(
                        fo,
                        "{}",
                        match format {
                            Format::Term => "\x1b\x28\x42\x1b\x5b\x6d",
                            Format::Html => "</span>",
                            Format::Json => panic!("ERR-012"),
                        }
                    )?;
                }
                write!(
                    fo,
                    "{}",
                    space.repeat(
                        maxlens[0] - connectors.len() * 4 - self.name.len()
                    )
                )?;
                match maxlens[1] + maxlens[2] {
                    0 => writeln!(fo)?,
                    _ => {
                        write!(fo, "{}│{}", space, space)?;
                        write!(fo, "{}", self.owner)?;
                        write!(
                            fo,
                            "{}",
                            space.repeat(maxlens[1] - self.owner.len())
                        )?;
                        if maxlens[1] > 0 && maxlens[2] > 0 {
                            write!(fo, "{}│{}", space, space)?;
                        }
                        self.fmt_comment(
                            fo, connectors, &mut 0, maxlens, format,
                        )?;
                    }
                }
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
                0 => write!(fo, "{}", space.repeat(4)),
                _ => write!(fo, "│{}", space.repeat(3)),
            }?;
            write!(
                fo,
                "{}",
                space.repeat(maxlens[0] - 4 - connectors.len() * 4)
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
