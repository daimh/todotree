use super::{Format, HTMLP, ROOT, Status, TodoError};
use std::cell::RefCell;
use std::cmp::{max, min};
use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::rc::Rc;

pub struct Todo {
    pub name: String,
    owner: String,
    comment: Vec<String>,
    pub dependencies: Vec<String>,
    /// the markdown file lines following each todo
    auxilaries: Vec<String>,
    children: Vec<Rc<RefCell<Todo>>>,
    /// the depth based on its deepest child
    depth: i32,
    pub status: Status,
}

impl Todo {
    pub fn new(
        mut name: String,
        owner: String,
        comment: Vec<String>,
        dependencies: Vec<String>,
        auxilaries: Vec<String>,
    ) -> Result<Self, TodoError> {
        let status = match name.starts_with("~") {
            true => {
                name = name.replace("~", "");
                Status::Completed
            }
            false => Status::Pending,
        };
        static SPECIALS: [char; 18] = [
            '!', '@', '$', '%', '%', '&', '(', ')', '-', '_', '=', '+', ':',
            '\'', '"', '.', '?', '/',
        ];
        if name != ROOT {
            if name.ends_with(ROOT) {
                return Err(TodoError {
                    msg: format!(
                        "ERR-018: Todo name '{}' should not end with '/'",
                        name
                    ),
                });
            }
            for c in name.chars() {
                if !SPECIALS.contains(&c)
                    && (c < 'a' || c > 'z')
                    && (c < 'A' || c > 'Z')
                    && (c < '0' || c > '9')
                {
                    return Err(TodoError {
                        msg: format!(
                            "ERR-001: Todo name '{}' contains unsupported \
                            character '{}', which is not alphabet, digit, \
							or {:?}",
                            name, c, SPECIALS
                        ),
                    });
                }
            }
        }
        let realauxl = match auxilaries.len() {
            0 => vec![String::new(); 1],
            _ => auxilaries,
        };
        Ok(Todo {
            name: name,
            owner: owner,
            comment: comment,
            status: status,
            dependencies: dependencies,
            auxilaries: realauxl,
            children: Vec::new(),
            depth: 0,
        })
    }

    pub fn build_tree(
        &mut self,
        visited: &mut BTreeSet<String>,
        map: &HashMap<String, Rc<RefCell<Todo>>>,
        maxlens: &mut [usize; 3],
        path: &mut BTreeSet<String>,
        depth: usize,
        screen_width: usize,
        hide: bool,
        dpth_limit: i32,
        format: &Format,
    ) -> Result<(), TodoError> {
        let mut notdonedeps: Vec<String> = vec![];
        for dep in &self.dependencies {
            let dep = dep.replace("~", "");
            if !path.insert(dep.clone()) {
                return Err(TodoError {
                    msg: format!(
                        "ERR-002: Todos '{:?}' has a dependency loop",
                        path
                    ),
                });
            }
            let child = map.get(&dep).unwrap_or_else(|| {
                panic!("ERR-003: Todo {} is missing in the markdown file", &dep)
            });
            let dep_notdone = child.borrow().status != Status::Completed;
            if dep_notdone {
                notdonedeps.push(dep.clone());
            }
            if dpth_limit <= 0 || dpth_limit > depth as i32 {
                if visited.insert(child.borrow().name.clone()) {
                    child.borrow_mut().build_tree(
                        visited,
                        map,
                        maxlens,
                        path,
                        depth + 1,
                        screen_width,
                        hide,
                        dpth_limit,
                        format,
                    )?;
                    let child_depth = child.borrow().depth;
                    self.depth = max(child_depth + 1, self.depth);
                    if (dpth_limit >= 0 || child_depth + dpth_limit >= 0)
                        && (dep_notdone || !hide)
                    {
                        self.children.push(Rc::clone(child));
                    }
                }
            }
            path.remove(&dep);
        }
        if notdonedeps.len() == 0 {
            if self.status != Status::Completed {
                self.status = Status::Actionable;
            }
        } else if self.status == Status::Completed {
            return Err(TodoError {
                msg: format!(
                    "ERR-004: Todo \"{}\" cannot be completed \
                    because its dependencies {:?} are not completed yet",
                    self.name, notdonedeps
                ),
            });
        }
        if self.name == ROOT {
            self.get_maxlens(maxlens, 0, screen_width)?;
        } else if self.dependencies.len() > 0
            && !self.name.ends_with(ROOT)
            && ((dpth_limit > 0 && dpth_limit == depth as i32)
                || (dpth_limit < 0 && self.depth + dpth_limit == 0))
        {
            self.name.push_str(ROOT)
        }
        Ok(())
    }

    pub fn get_maxlens(
        &mut self,
        maxlens: &mut [usize; 3],
        depth: usize,
        screen_width: usize,
    ) -> Result<(), TodoError> {
        for child in &self.children {
            child
                .borrow_mut()
                .get_maxlens(maxlens, depth + 1, screen_width)?;
        }
        if self.name == ROOT {
            if maxlens[1] > 0 {
                self.owner = String::from("OWNER")
            }
            if maxlens[2] > 0 {
                self.comment = vec![String::from("COMMENT"); 1];
            }
        }
        maxlens[0] = max(maxlens[0], depth * 4 + self.name.len());
        maxlens[1] = max(maxlens[1], self.owner.len());
        for line in &self.comment {
            maxlens[2] = max(maxlens[2], line.len());
        }
        if self.name == ROOT {
            if screen_width <= maxlens[0] + maxlens[1] + 8 {
                return Err(TodoError {
                    msg: format!(
                        "ERR-005: Screen width is {}, but this todotree \
                        needs at least {} columns",
                        screen_width,
                        maxlens[0] + maxlens[1] + 8
                    ),
                });
            }
            maxlens[2] =
                min(maxlens[2], screen_width - maxlens[0] - maxlens[1] - 8);
        }
        Ok(())
    }

    pub fn fmt_tree(
        &self,
        fo: &mut fmt::Formatter<'_>,
        connectors: &mut Vec<bool>,
        visited: &mut BTreeSet<String>,
        maxlens: &[usize; 3],
        format: &Format,
    ) -> fmt::Result {
        let space: String;
        match format {
            Format::Md => {
                space = String::from("Panic");
                if self.name != "/" {
                    write!(fo, "# ")?;
                    if self.status == Status::Completed {
                        write!(fo, "~")?;
                    }
                    writeln!(fo, "{}", self.name)?;
                    if self.owner != "" {
                        writeln!(fo, "- @ {}", self.owner)?;
                    }
                    if self.dependencies.len() > 0 {
                        let normalized = self
                            .dependencies
                            .iter()
                            .map(|x| x.replace("~", ""))
                            .collect::<Vec<String>>();
                        writeln!(fo, "- : {}", normalized.join(" "))?;
                    }
                    for comt in &self.comment {
                        writeln!(fo, "- % {}", comt)?;
                    }
                    for ln in &self.auxilaries {
                        writeln!(fo, "{}", ln)?;
                    }
                }
            }
            Format::Json => {
                space = " ".repeat(connectors.len() * 4);
                writeln!(fo, "{}{{", space)?;
                writeln!(fo, "{}  \"name\": \"{}\",", space, self.name)?;
                writeln!(fo, "{}  \"status\": \"{}\",", space, self.status)?;
                if maxlens[1] > 0 {
                    writeln!(fo, "{}  \"owner\": \"{}\",", space, self.owner)?;
                }
                if maxlens[2] > 0 && self.comment.len() > 0 {
                    writeln!(
                        fo,
                        "{}  \"comment\": \"{}\",",
                        space, self.comment[0]
                    )?;
                }
                writeln!(fo, "{}  \"dependencies\": [", space)?;
            }
            Format::Term => {
                space = String::from(" ");
                let bol = String::new();
                self.fmt_connector(fo, connectors, &space, &bol)?;
                match self.status {
                    Status::Completed => write!(fo, "\x1b\x5b\x33\x34\x6d")?,
                    Status::Actionable => write!(fo, "\x1b\x5b\x33\x31\x6d")?,
                    Status::Pending => (),
                }
                write!(fo, "{}", self.name)?;
                if self.status != Status::Pending {
                    write!(fo, "\x1b\x28\x42\x1b\x5b\x6d")?;
                }
                let eol = String::from("\n");
                self.fmt_table(fo, connectors, maxlens, &space, &bol, &eol)?;
            }
            Format::Html => {
                space = String::from("&nbsp;");
                let bol = String::from(HTMLP);
                let eol = String::from("</p>\n");
                self.fmt_connector(fo, connectors, &space, &bol)?;
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
                self.fmt_table(fo, connectors, maxlens, &space, &bol, &eol)?;
            }
        }
        for (pos, child) in self.children.iter().enumerate() {
            if visited.insert(child.borrow().name.clone()) {
                connectors.push(pos + 1 == self.children.len());
                if pos > 0 && *format == Format::Json {
                    writeln!(fo, "{}    ,", space)?;
                }
                child
                    .borrow()
                    .fmt_tree(fo, connectors, visited, maxlens, format)?;
                connectors.pop();
            }
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
        bol: &String,
    ) -> fmt::Result {
        write!(fo, "{}", bol)?;
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
        maxlens: &[usize; 3],
        space: &String,
        bol: &String,
        eol: &String,
    ) -> fmt::Result {
        if maxlens[1] + maxlens[2] == 0 {
            write!(fo, "{}", eol)?;
        } else {
            write!(
                fo,
                "{}",
                space.repeat(
                    maxlens[0] - connectors.len() * 4 - self.name.len()
                )
            )?;
            write!(fo, "{}│{}", space, space)?;
            if maxlens[1] > 0 {
                write!(
                    fo,
                    "{}{}│",
                    self.owner,
                    space.repeat(1 + maxlens[1] - self.owner.len())
                )?;
                if maxlens[2] > 0 {
                    write!(fo, "{}", space)?;
                }
            }
            if maxlens[2] > 0 {
                self.fmt_comment(fo, connectors, maxlens, space, bol, eol)?;
            } else {
                write!(fo, "{}", eol)?;
                self.fmt_cont_comment_or_dash(
                    fo, connectors, maxlens, &space, &bol, false,
                )?;
            }
            write!(fo, "{}", eol)?;
        }
        Ok(())
    }

    fn fmt_cont_comment_or_dash(
        &self,
        fo: &mut fmt::Formatter<'_>,
        connectors: &Vec<bool>,
        maxlens: &[usize; 3],
        space: &String,
        bol: &String,
        iscomment: bool,
    ) -> fmt::Result {
        write!(fo, "{}", bol)?;
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
        if iscomment {
            write!(fo, "{}│{}", space, space)?;
            write!(fo, "{}", space.repeat(maxlens[1]))?;
            if maxlens[1] > 0 && maxlens[2] > 0 {
                write!(fo, "{}│{}", space, space)?;
            }
        } else {
            let mut last = self.children.len() == 0;
            if last {
                for b in connectors {
                    last = *b;
                    if !last {
                        break;
                    }
                }
            }
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
        }
        Ok(())
    }

    fn fmt_comment(
        &self,
        fo: &mut fmt::Formatter<'_>,
        connectors: &Vec<bool>,
        maxlens: &[usize; 3],
        space: &String,
        bol: &String,
        eol: &String,
    ) -> fmt::Result {
        let comt = match self.comment.len() {
            0 => &vec![String::new(); 1],
            _ => &self.comment,
        };
		let dgt_width = comt.len().to_string().len();
		let seq_width = match self.comment.len() {
			0 | 1 => 0,
			_ => dgt_width + 2,
		};
		let cmt_width = maxlens[2] - seq_width;
        for (idx, line) in comt.iter().enumerate() {
            if idx > 0 {
                self.fmt_cont_comment_or_dash(
                    fo, connectors, maxlens, space, bol, true,
                )?;
            }
            let mut start = 0;
            loop {
                let slen = min(line.len() - start, cmt_width);
				if seq_width > 0 {
					match start {
						0 => write!(fo, "{:0>dgt_width$}.{}", idx+1, space),
						_ => write!(fo, "{}", space.repeat(seq_width)),
					}?;
				}
                write!(fo, "{}", &line[start..start + slen])?;
                write!(fo, "{}", space.repeat(cmt_width - slen))?;
                write!(fo, "{}│{}", space, eol)?;
                start = start + slen;
                if start >= line.len() {
                    break;
                }
                self.fmt_cont_comment_or_dash(
                    fo, connectors, maxlens, space, bol, true,
                )?;
            }
        }
        self.fmt_cont_comment_or_dash(
            fo, connectors, maxlens, space, bol, false,
        )
    }
}
