use super::{Format, HTMLP, ROOT, Status, TodoError};
use std::cell::RefCell;
use std::cmp::{max, min};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::rc::Rc;

#[derive(PartialEq)]
enum Location {
    Top,
    Mid,
    Bottom,
}

pub struct Todo {
    pub name: String,
    pub owner: String,
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
        let status = if name.starts_with("~") {
            name = name.replace("~", "");
            Status::Completed
        } else {
            Status::Pending
        };
        static SPECIALS: [char; 18] = [
            '!', '@', '$', '%', '%', '&', '(', ')', '-', '_', '=', '+', ':',
            '\'', '"', '.', '?', '/',
        ];
        if name != ROOT {
            if name.ends_with(ROOT) {
                return Err(TodoError::Input(format!(
                    "ERR-018: TODO name '{}' should not end with '/'",
                    name
                )));
            }
            for c in name.chars() {
                if !SPECIALS.contains(&c)
                    && (c < 'a' || c > 'z')
                    && (c < 'A' || c > 'Z')
                    && (c < '0' || c > '9')
                {
                    return Err(TodoError::Input(format!(
                        "ERR-001: TODO name '{}' contains unsupported \
                            character '{}', which is not alphabet, digit, \
                            or {:?}",
                        name, c, SPECIALS
                    )));
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
        map: &BTreeMap<String, Rc<RefCell<Todo>>>,
        maxlens: &mut [usize; 3],
        path: &mut BTreeSet<String>,
        depth: usize,
        screen_width: usize,
        hide_done: bool,
        hide_owner: bool,
        dpth_limit: i32,
        format: &Format,
        owners: &mut BTreeMap<String, bool>,
    ) -> Result<bool, TodoError> {
        let mut own_me = if owners.len() == 0 {
            true
        } else if owners.contains_key(&self.owner) {
            owners.insert(self.owner.clone(), true);
            true
        } else {
            false
        };
        if hide_owner {
            self.owner = String::new();
        }
        let mut notdonedeps: Vec<String> = vec![];
        for dep in &self.dependencies {
            let dep = dep.replace("~", "");
            if !path.insert(dep.clone()) {
                return Err(TodoError::Input(format!(
                    "ERR-002: TODOs '{:?}' has a dependency loop",
                    path
                )));
            }
            let child = match map.get(&dep) {
                Some(m) => m,
                None => {
                    return Err(TodoError::Input(format!(
                        "ERR-003: TODO '{}' is missing in the markdown file",
                        &dep
                    )));
                }
            };
            let dep_notdone = child.borrow().status != Status::Completed;
            if dep_notdone {
                notdonedeps.push(dep.clone());
            }
            if dpth_limit <= 0 || dpth_limit > depth as i32 {
                if visited.insert(child.borrow().name.clone()) {
                    let own_child = child.borrow_mut().build_tree(
                        visited,
                        map,
                        maxlens,
                        path,
                        depth + 1,
                        screen_width,
                        hide_done,
                        hide_owner,
                        dpth_limit,
                        format,
                        owners,
                    )?;
                    if own_child {
                        own_me = true;
                        let child_depth = child.borrow().depth;
                        self.depth = max(child_depth + 1, self.depth);
                        if (dpth_limit >= 0 || child_depth + dpth_limit >= 0)
                            && (dep_notdone || !hide_done)
                        {
                            self.children.push(Rc::clone(child));
                        }
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
            return Err(TodoError::Input(format!(
                "ERR-004: TODO \"{}\" cannot be completed \
                    because its dependencies {:?} are not completed yet",
                self.name, notdonedeps
            )));
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
        Ok(own_me)
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
                self.owner = "OWNER".to_string()
            }
            if maxlens[2] > 0 {
                self.comment = vec!["COMMENT".to_string(); 1];
            }
        }
        maxlens[0] = max(maxlens[0], depth * 4 + self.name.len());
        maxlens[1] = max(maxlens[1], self.owner.len());
        for line in &self.comment {
            maxlens[2] = max(maxlens[2], line.len());
        }
        if self.comment.len() > 1 {
            maxlens[2] += self.comment.len().to_string().len() + 2;
        }
        if self.name == ROOT {
            if screen_width <= maxlens[0] + maxlens[1] + 8 {
                return Err(TodoError::Input(format!(
                    "ERR-005: Screen width is {}, but this todotree \
                        needs at least {} columns",
                    screen_width,
                    maxlens[0] + maxlens[1] + 9
                )));
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
        no_color: bool,
        reverse: bool,
    ) -> fmt::Result {
        let space = match format {
            Format::Md => "PANIC",
            Format::Json => &" ".repeat(connectors.len() * 4),
            Format::Term => " ",
            Format::Html => "&nbsp;",
        };
        let (bol, eol) = match format {
            Format::Term => ("", "\n"),
            Format::Html => (HTMLP, "</p>\n"),
            _ => ("", ""),
        };
        if (*format == Format::Html || *format == Format::Term)
            && self.name == ROOT
            && maxlens[1] + maxlens[2] > 0
        {
            self.fmt_row_separator(
                fo,
                connectors,
                maxlens,
                space,
                bol,
                eol,
                reverse,
                &Location::Top,
            )?;
        }
        let children_iter = self.children.iter().enumerate();
        if reverse {
            for (pos, child) in children_iter.clone().rev() {
                if visited.insert(child.borrow().name.clone()) {
                    connectors.push(pos + 1 == self.children.len());
                    child.borrow().fmt_tree(
                        fo, connectors, visited, maxlens, format, no_color,
                        reverse,
                    )?;
                    connectors.pop();
                }
            }
        }
        match format {
            Format::Md => {
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
                let boc = if no_color {
                    ""
                } else {
                    match self.status {
                        Status::Completed => "\x1b[34m", // blue foreground
                        Status::Actionable => "\x1b[31m", // red foreground
                        Status::Pending => "",
                    }
                };
                let eoc = if !no_color && self.status != Status::Pending {
                    "\x1b(B\x1b[m"
                } else {
                    ""
                };
                self.fmt_table(
                    fo, connectors, maxlens, space, bol, eol, boc, eoc, reverse,
                )?;
            }
            Format::Html => {
                let boc = if no_color {
                    ""
                } else {
                    match self.status {
                        Status::Completed => "<span style='color:blue'>",
                        Status::Actionable => "<span style='color:red'>",
                        Status::Pending => "",
                    }
                };
                let eoc = if !no_color && self.status != Status::Pending {
                    "</span>"
                } else {
                    ""
                };
                self.fmt_table(
                    fo, connectors, maxlens, space, bol, eol, boc, eoc, reverse,
                )?;
            }
        }
        if !reverse {
            for (pos, child) in children_iter {
                if visited.insert(child.borrow().name.clone()) {
                    connectors.push(pos + 1 == self.children.len());
                    if pos > 0 && *format == Format::Json {
                        writeln!(fo, "{}    ,", space)?;
                    }
                    child.borrow().fmt_tree(
                        fo, connectors, visited, maxlens, format, no_color,
                        reverse,
                    )?;
                    connectors.pop();
                }
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
        space: &str,
        bol: &str,
        boc: &str,
        eoc: &str,
        reverse: bool,
    ) -> fmt::Result {
        write!(fo, "{}", bol)?;
        for (pos, cn) in connectors.iter().enumerate() {
            if *cn {
                if pos + 1 < connectors.len() {
                    write!(fo, "{}", space.repeat(4))?;
                } else if reverse {
                    write!(fo, "┌──{}", space)?;
                } else {
                    write!(fo, "└──{}", space)?;
                }
            } else if pos + 1 < connectors.len() {
                write!(fo, "│{}", space.repeat(3))?;
            } else {
                write!(fo, "├──{}", space)?;
            }
        }
        write!(fo, "{}{}{}", boc, self.name, eoc)
    }

    fn fmt_table(
        &self,
        fo: &mut fmt::Formatter<'_>,
        connectors: &mut Vec<bool>,
        maxlens: &[usize; 3],
        space: &str,
        bol: &str,
        eol: &str,
        boc: &str,
        eoc: &str,
        reverse: bool,
    ) -> fmt::Result {
        if maxlens[1] + maxlens[2] == 0 {
            self.fmt_connector(fo, connectors, space, bol, boc, eoc, reverse)?;
            return write!(fo, "{}", eol);
        }
        self.fmt_connector(fo, connectors, space, bol, boc, eoc, reverse)?;
        write!(
            fo,
            "{}",
            space.repeat(maxlens[0] - connectors.len() * 4 - self.name.len())
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
        let location = if reverse {
            if self.name == ROOT {
                Location::Bottom
            } else {
                Location::Mid
            }
        } else {
            let mut last = self.children.len() == 0;
            if last {
                for b in connectors.iter() {
                    last = last && *b;
                    if !last {
                        break;
                    }
                }
            }
            if last {
                Location::Bottom
            } else {
                Location::Mid
            }
        };
        match maxlens[2] {
            0 => write!(fo, "{}", eol)?,
            _ => self.fmt_comment(
                fo, connectors, maxlens, space, bol, eol, reverse, &location,
            )?,
        }
        self.fmt_row_separator(
            fo, connectors, maxlens, space, bol, eol, reverse, &location,
        )
    }

    fn fmt_space_before_table(
        &self,
        fo: &mut fmt::Formatter<'_>,
        connectors: &Vec<bool>,
        maxlens: &[usize; 3],
        space: &str,
        bol: &str,
        reverse: bool,
        location: &Location,
    ) -> fmt::Result {
        write!(fo, "{}", bol)?;
        for (i, b) in connectors.iter().enumerate() {
            if reverse && i + 1 == connectors.len() {
                break;
            }
            if *b {
                write!(fo, "{}", space)?;
            } else {
                write!(fo, "│")?;
            }
            write!(fo, "{}", space.repeat(3))?;
        }
        if reverse {
            if connectors.len() > 0 {
                write!(fo, "│{}", space.repeat(4))?
            } else {
                write!(fo, "{}", space)?
            }
        } else if self.children.len() == 0 || *location == Location::Top {
            write!(fo, "{}", space)?;
        } else {
            write!(fo, "│")?;
        }
        write!(
            fo,
            "{}",
            space.repeat(maxlens[0] - 1 - connectors.len() * 4)
        )
    }

    fn fmt_row_separator(
        &self,
        fo: &mut fmt::Formatter<'_>,
        connectors: &Vec<bool>,
        maxlens: &[usize; 3],
        space: &str,
        bol: &str,
        eol: &str,
        reverse: bool,
        location: &Location,
    ) -> fmt::Result {
        self.fmt_space_before_table(
            fo, connectors, maxlens, space, bol, reverse, location,
        )?;
        let (cl, cm, cr) = match location {
            Location::Top => ("┌", "┬", "┐"),
            Location::Mid => ("├", "┼", "┤"),
            Location::Bottom => ("└", "┴", "┘"),
        };
        write!(fo, "{}{}─{}", space, cl, "─".repeat(maxlens[1]))?;
        if maxlens[1] > 0 && maxlens[2] > 0 {
            write!(fo, "─{}─", cm)?;
        }
        write!(fo, "{}─{}{}", "─".repeat(maxlens[2]), cr, eol)
    }

    fn fmt_cont_comment(
        &self,
        fo: &mut fmt::Formatter<'_>,
        connectors: &Vec<bool>,
        maxlens: &[usize; 3],
        space: &str,
        bol: &str,
        reverse: bool,
        location: &Location,
    ) -> fmt::Result {
        self.fmt_space_before_table(
            fo, connectors, maxlens, space, bol, reverse, location,
        )?;
        write!(fo, "{}│{}", space, space)?;
        write!(fo, "{}", space.repeat(maxlens[1]))?;
        if maxlens[1] > 0 && maxlens[2] > 0 {
            write!(fo, "{}│{}", space, space)?;
        }
        Ok(())
    }

    fn fmt_comment(
        &self,
        fo: &mut fmt::Formatter<'_>,
        connectors: &Vec<bool>,
        maxlens: &[usize; 3],
        space: &str,
        bol: &str,
        eol: &str,
        reverse: bool,
        location: &Location,
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
                self.fmt_cont_comment(
                    fo, connectors, maxlens, space, bol, reverse, location,
                )?;
            }
            let mut start = 0;
            loop {
                let slen = min(line.len() - start, cmt_width);
                if seq_width > 0 {
                    match start {
                        0 => write!(fo, "{:0>dgt_width$}.{}", idx + 1, space),
                        _ => write!(fo, "{}", space.repeat(seq_width)),
                    }?;
                }
                write!(fo, "{}", &line[start..start + slen])?;
                write!(fo, "{}", space.repeat(cmt_width - slen))?;
                write!(fo, "{}│{}", space, eol)?;
                start += slen;
                if start >= line.len() {
                    break;
                }
                self.fmt_cont_comment(
                    fo, connectors, maxlens, space, bol, reverse, location,
                )?;
            }
        }
        Ok(())
    }
}
