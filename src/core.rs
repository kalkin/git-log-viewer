use configparser::ini::Ini;
use lazy_static::lazy_static;
use regex::Regex;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use git_subtrees_improved::{changed_modules, SubtreeConfig};
use git_wrapper::{git_cmd_out, is_ancestor};
use posix_errors::PosixError;

lazy_static! {
    static ref CONFIG: Ini = config();
}

#[macro_export]
macro_rules! regex {
    ($r:literal) => {
        Regex::new($r).expect("Valid RegEx")
    };
}

macro_rules! next_string {
    ($split:expr) => {
        $split.next().expect("Another split").to_string();
    };
}

#[derive(derive_more::Display, derive_more::FromStr, Clone, Eq, PartialEq)]
#[display(fmt = "{}", self.0)]
pub struct Oid(pub String);

#[derive(derive_more::Display, derive_more::FromStr, Eq, PartialEq, Clone)]
#[display(fmt = "{}", self.0)]
pub struct GitRef(pub String);

#[derive(Clone)]
pub struct Commit {
    id: Oid,
    short_id: String,
    author_name: String,
    author_email: String,
    author_date: String,
    author_rel_date: String,
    committer_name: String,
    committer_email: String,
    committer_date: String,
    committer_rel_date: String,
    subject: String,
    body: String,

    icon: String,

    bellow: Option<Oid>,
    children: Vec<Oid>,
    is_commit_link: bool,
    is_fork_point: bool,
    is_head: bool,
    is_merge: bool,
    branches: Vec<GitRef>,
    references: Vec<GitRef>,
    tags: Vec<GitRef>,
}

impl Commit {
    pub fn author_name(&self) -> &String {
        &self.author_name
    }
    pub fn author_email(&self) -> &String {
        &self.author_email
    }
    pub fn author_date(&self) -> &String {
        &self.author_date
    }

    pub fn author_rel_date(&self) -> &String {
        &self.author_rel_date
    }

    pub fn bellow(&self) -> Option<&Oid> {
        self.bellow.as_ref()
    }

    pub fn body(&self) -> &String {
        &self.body
    }

    pub fn branches(&self) -> &Vec<GitRef> {
        &self.branches
    }

    pub fn committer_name(&self) -> &String {
        &self.committer_name
    }
    pub fn committer_email(&self) -> &String {
        &self.committer_email
    }
    pub fn committer_date(&self) -> &String {
        &self.committer_date
    }

    pub fn committer_rel_date(&self) -> &String {
        &self.committer_rel_date
    }

    pub fn children(&self) -> &Vec<Oid> {
        &self.children
    }

    pub fn id(&self) -> &Oid {
        &self.id
    }

    pub fn icon(&self) -> &String {
        &self.icon
    }

    pub fn is_head(&self) -> bool {
        self.is_head
    }
    pub fn is_fork_point(&self) -> bool {
        self.is_fork_point
    }
    pub fn is_merge(&self) -> bool {
        self.bellow.is_some() && !self.children.is_empty()
    }
    pub fn is_commit_link(&self) -> bool {
        self.is_commit_link
    }

    pub fn references(&self) -> &Vec<GitRef> {
        &self.references
    }
    pub fn short_id(&self) -> &String {
        &self.short_id
    }
    pub fn subject(&self) -> &String {
        &self.subject
    }

    pub fn tags(&self) -> &Vec<GitRef> {
        &self.tags
    }
}

const REV_FORMAT: &str =
    "--format=%x1f%H%x1f%h%x1f%P%x1f%D%x1f%aN%x1f%aE%x1f%aI%x1f%ar%x1f%cN%x1f%cE%x1f%cI%x1f%cr%x1f%s%x1f%b%x1e";

impl Commit {
    pub fn new(data: &str, is_commit_link: bool, is_fork_point: bool) -> Self {
        let mut split = data.split('\x1f');
        split.next(); // skip commit: XXXX line
        let id = Oid {
            0: next_string!(split),
        };

        let short_id = next_string!(split);
        let mut parents_record: Vec<&str> = split.next().unwrap().split(' ').collect();
        let references_record = next_string!(split);

        let author_name = next_string!(split);
        let author_email = next_string!(split);
        let author_date = next_string!(split);
        let author_rel_date = next_string!(split);

        let committer_name = next_string!(split);
        let committer_email = next_string!(split);
        let committer_date = next_string!(split);
        let committer_rel_date = next_string!(split);
        let subject = next_string!(split);
        let body = next_string!(split);

        let mut is_head = false;

        let mut references: Vec<GitRef> = Vec::new();
        let mut branches: Vec<GitRef> = Vec::new();
        let mut tags: Vec<GitRef> = Vec::new();
        for s in references_record.split(", ") {
            if s.is_empty() {
                continue;
            } else if s == "HEAD" {
                is_head = true
            } else if s.starts_with("HEAD -> ") {
                is_head = true;
                let split_2: Vec<&str> = s.splitn(2, " -> ").collect();
                let branch = split_2[1].to_string();
                branches.push(GitRef(branch.clone()));
                references.push(GitRef(branch));
            } else if s.starts_with("tag: ") {
                let split_2: Vec<&str> = s.splitn(2, ": ").collect();
                let tag = split_2[1].to_string();
                tags.push(GitRef(tag.clone()));
                references.push(GitRef(tag));
            } else {
                let branch = s.to_string();
                branches.push(GitRef(branch.clone()));
                references.push(GitRef(branch));
            }
        }

        let is_merge = parents_record.len() >= 2;

        let bellow;
        if parents_record.is_empty() {
            bellow = None;
        } else {
            bellow = Some(Oid(parents_record.remove(0).to_string()));
        }

        let mut children = Vec::new();
        for c in parents_record {
            children.push(Oid(c.to_string()));
        }

        let mut icon = " ".to_string();
        for (reg, c) in REGEXES.iter() {
            if reg.is_match(&subject) {
                icon = c.to_string();
                break;
            }
        }

        Commit {
            id,
            short_id,

            author_name,
            author_date,
            author_email,
            author_rel_date,

            committer_name,
            committer_email,
            committer_date,
            committer_rel_date,

            subject,
            body,

            icon,

            bellow,
            children,

            is_commit_link,
            is_fork_point,
            is_head,
            is_merge,
            branches,
            references,
            tags,
        }
    }

    fn calc_is_fork_point(&mut self, working_dir: &str, above: &Option<&Commit>) {
        if let Some(c) = above {
            if !c.children.is_empty() && c.children[0] != self.id {
                let parent_child = c.children[0].to_string();
                if c.is_merge {
                    self.is_fork_point = is_ancestor(working_dir, &self.id.0, &parent_child)
                        .expect("Execute merge-base --is-ancestor");
                }
            }
        }
    }
}

pub fn history_length(
    working_dir: &str,
    rev_range: &str,
    paths: Vec<&str>,
) -> Result<usize, PosixError> {
    let mut args = vec!["--first-parent", "--count", rev_range];
    if !paths.is_empty() {
        args.push("--");
        for p in paths {
            args.push(p);
        }
    }

    let output = git_wrapper::rev_list(working_dir, args)?;
    Ok(output
        .parse::<usize>()
        .expect("Failed to parse commit length"))
}

pub fn commits_for_range<T: AsRef<str>>(
    working_dir: &str,
    rev_range: &str,
    above_commit: Option<&Commit>,
    paths: &[T],
    skip: Option<usize>,
    max: Option<usize>,
) -> Result<Vec<Commit>, PosixError> {
    let mut args = vec!["--first-parent", REV_FORMAT];

    let tmp;
    if let Some(val) = skip {
        tmp = format!("--skip={}", val);
        args.push(&tmp);
    }

    let tmp2;
    if let Some(val) = max {
        tmp2 = format!("--max-count={}", val);
        args.push(&tmp2);
    }

    args.push(rev_range);

    if !paths.is_empty() {
        args.push("--");
        for p in paths {
            args.push(p.as_ref());
        }
    }

    let output = git_wrapper::rev_list(working_dir, args)?;
    let lines = output.split('\u{1e}');
    let mut result: Vec<Commit> = Vec::new();
    let mut above = above_commit;
    for data in lines {
        if data.is_empty() {
            break;
        }
        let mut commit = Commit::new(data, false, false);
        commit.calc_is_fork_point(working_dir, &above);
        result.push(commit);
        above = result.last();
    }
    Ok(result)
}

pub fn child_history(
    working_dir: &str,
    commit: &Commit,
    subtree_modules: &[SubtreeConfig],
) -> Vec<Commit> {
    let bellow = commit.bellow.as_ref().expect("Expected merge commit");
    let first_child = commit.children.get(0).expect("Expected merge commit");
    let end = merge_base(working_dir, bellow, first_child).expect("merge-base invocation");
    let revision;
    if let Some(v) = &end {
        if v == first_child {
            revision = first_child.0.clone();
        } else {
            revision = format!("{}..{}", v.0, first_child.0);
        }
    } else {
        revision = first_child.0.clone();
    }
    let above_commit = commit;
    let paths: &[&str] = &[];
    let mut result = commits_for_range(
        working_dir,
        revision.as_str(),
        Some(above_commit),
        paths,
        None,
        None,
    )
    .unwrap_or_else(|_| panic!("Expected child commits for range {}", revision));
    let end_commit = result
        .last()
        .unwrap_or_else(|| panic!("No child commits for range {}", revision));
    if end.is_some()
        && end_commit.bellow.is_some()
        && end_commit.bellow.as_ref().expect("Expected merge commit") != bellow
    {
        let mut link = to_commit(
            working_dir,
            end_commit.bellow.as_ref().expect("Expected merge commit"),
            true,
            false,
        );

        link.calc_is_fork_point(working_dir, &Some(end_commit));
        result.push(link);
    }

    result
}

fn to_commit(working_dir: &str, oid: &Oid, is_commit_link: bool, is_fork_point: bool) -> Commit {
    let output = git_cmd_out(
        working_dir.to_string(),
        vec!["rev-list", REV_FORMAT, "-1", &oid.0],
    );
    let tmp = String::from_utf8(output.unwrap().stdout);
    let lines: Vec<&str> = tmp.as_ref().expect("Valid UTF-8").lines().collect();
    // XXX FIXME lines? really?
    assert!(lines.len() >= 2, "Did not got enough data for {}", oid);
    Commit::new(lines.get(1).unwrap(), is_commit_link, is_fork_point)
}

pub fn merge_base(working_dir: &str, p1: &Oid, p2: &Oid) -> Result<Option<Oid>, PosixError> {
    let output =
        git_wrapper::git_cmd_out(working_dir.to_string(), vec!["merge-base", &p1.0, &p2.0]);
    let tmp = String::from_utf8(output?.stdout)
        .expect("Valid UTF-8")
        .trim_end()
        .to_string();
    if tmp.is_empty() {
        Ok(None)
    } else {
        Ok(Some(Oid { 0: tmp }))
    }
}

lazy_static! {
    static ref REGEXES: Vec<(Regex, &'static str)> = vec![
        (regex!(r"(?i)^Revert:?\s*"), " "),
        (regex!(r"(?i)^archive:?\s*"), "\u{f53b} "),
        (regex!(r"(?i)^issue:?\s*"), "\u{f145} "),
        (regex!(r"(?i)^BREAKING CHANGE:?\s*"), "⚠ "),
        (regex!(r"(?i)^fixup!\s+"), "\u{f0e3} "),
        (regex!(r"(?i)^ADD:\s?[a-z0-9]+"), " "),
        (regex!(r"(?i)^ref(actor)?:?\s*"), "↺ "),
        (regex!(r"(?i)^lang:?\s*"), "\u{fac9}"),
        (regex!(r"(?i)^deps(\(.+\))?:?\s*"), "\u{f487} "),
        (regex!(r"(?i)^config:?\s*"), "\u{f462} "),
        (regex!(r"(?i)^test(\(.+\))?:?\s*"), "\u{f45e} "),
        (regex!(r"(?i)^ci(\(.+\))?:?\s*"), "\u{f085} "),
        (regex!(r"(?i)^perf(\(.+\))?:?\s*"), "\u{f9c4}"),
        (
            regex!(r"(?i)^(bug)?fix(ing|ed)?(\(.+\))?[/:\s]+"),
            "\u{f188} "
        ),
        (regex!(r"(?i)^doc(s|umentation)?:?\s*"), "✎ "),
        (regex!(r"(?i)^improve(ment)?:?\s*"), "\u{e370} "),
        (regex!(r"(?i)^CHANGE/?:?\s*"), "\u{e370} "),
        (regex!(r"(?i)^hotfix:?\s*"), "\u{f490} "),
        (regex!(r"(?i)^feat:?\s*"), "➕"),
        (regex!(r"(?i)^add:?\s*"), "➕"),
        (regex!(r"(?i)^(release|bump):?\s*"), "\u{f412} "),
        (regex!(r"(?i)^build:?\s*"), "🔨"),
        (regex!(r"(?i).*\bchangelog\b.*"), "✎ "),
        (regex!(r"(?i)^refactor:?\s*"), "↺ "),
        (regex!(r"(?i)^.* Import .*"), "⮈ "),
        (regex!(r"(?i)^Split .*"), "\u{f403} "),
        (regex!(r"(?i)^Remove:?\s+.*"), "\u{f48e} "),
        (regex!(r"(?i)^Update :\w+.*"), "\u{f419} "),
        (regex!(r"(?i)^style:?\s*"), "♥ "),
        (regex!(r"(?i)^DONE:?\s?[a-z0-9]+"), "\u{f41d} "),
        (regex!(r"(?i)^rename?\s*"), "\u{f044} "),
        (regex!(r"(?i).*"), "  "),
    ];
}

fn config() -> Ini {
    let xdg_dirs = xdg::BaseDirectories::with_prefix("glv").expect("Expected BaseDirectories");
    let mut result = Ini::new();
    match xdg_dirs.find_config_file("config") {
        None => {}
        Some(config_path) => {
            let path = config_path
                .to_str()
                .expect("A path convertible to an UTF-8 string");
            result.load(path).expect("Loaded INI file");
        }
    }
    result
}

// I'm not proud of this code. Ohh Omnissiah be merciful on my soul‼
pub fn adjust_string(text: &str, len: usize) -> String {
    assert!(len > 0, "Minimal length should be 1");
    let actual = unicode_width::UnicodeWidthStr::width(text);
    let expected = len;
    let mut result = String::from(text);
    if actual < len {
        let end = len - actual;
        for _ in 0..end {
            result.push(' ');
        }
    } else if actual > len {
        let words = text.unicode_words().collect::<Vec<&str>>();
        result = "".to_string();
        for w in words {
            let actual = UnicodeWidthStr::width(result.as_str()) + UnicodeWidthStr::width(w);
            if actual > expected {
                break;
            }
            result.push_str(w);
            result.push(' ');
        }

        if result.is_empty() {
            let words = text.unicode_words().collect::<Vec<&str>>();
            result.push_str(words.get(0).unwrap());
        }

        let actual = UnicodeWidthStr::width(result.as_str());
        if actual > expected {
            let mut tmp = String::new();
            let mut i = 0;
            for g in result.as_str().graphemes(true) {
                tmp.push_str(g);
                i += 1;
                if i == expected - 1 {
                    break;
                }
            }
            result = tmp;
            result.push('…');
        } else {
            let end = expected - actual;
            for _ in 0..end {
                result.push(' ');
            }
        }
        return result;
    }
    result
}

pub fn author_name_width() -> usize {
    match CONFIG.getuint("history", "author_name_width") {
        Ok(o) => match o {
            None => 10,
            Some(v) => v as usize,
        },
        Err(_) => panic!("Error while parsing history.author_name_width"),
    }
}

pub fn author_rel_date_width() -> usize {
    match CONFIG.getuint("history", "author_rel_date_width") {
        Ok(o) => match o {
            None => 0,
            Some(v) => v as usize,
        },
        Err(_) => panic!("Error while parsing history.author_rel_name_width"),
    }
}

pub fn modules_width() -> usize {
    match CONFIG.getuint("history", "modules_width") {
        Ok(o) => match o {
            None => 35,
            Some(v) => v as usize,
        },
        Err(_) => panic!("Error while parsing history.modules_width"),
    }
}
