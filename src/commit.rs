use lazy_static::lazy_static;
use regex::Regex;

use git_wrapper::git_cmd_out;
use posix_errors::PosixError;

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
    is_head: bool,
    is_merge: bool,
    branches: Vec<GitRef>,
    references: Vec<GitRef>,
    tags: Vec<GitRef>,
}

impl Commit {
    #[must_use]
    pub fn author_name(&self) -> &String {
        &self.author_name
    }
    #[must_use]
    pub fn author_email(&self) -> &String {
        &self.author_email
    }
    #[must_use]
    pub fn author_date(&self) -> &String {
        &self.author_date
    }

    #[must_use]
    pub fn author_rel_date(&self) -> &String {
        &self.author_rel_date
    }

    #[must_use]
    pub fn bellow(&self) -> Option<&Oid> {
        self.bellow.as_ref()
    }

    #[allow(dead_code)]
    #[must_use]
    pub fn branches(&self) -> &Vec<GitRef> {
        &self.branches
    }

    #[must_use]
    pub fn body(&self) -> &String {
        &self.body
    }

    #[must_use]
    pub fn committer_name(&self) -> &String {
        &self.committer_name
    }
    #[must_use]
    pub fn committer_email(&self) -> &String {
        &self.committer_email
    }
    #[must_use]
    pub fn committer_date(&self) -> &String {
        &self.committer_date
    }

    #[must_use]
    pub fn children(&self) -> &Vec<Oid> {
        &self.children
    }

    #[must_use]
    pub fn id(&self) -> &Oid {
        &self.id
    }

    #[must_use]
    pub fn icon(&self) -> &String {
        &self.icon
    }

    #[allow(dead_code)]
    #[must_use]
    pub fn is_head(&self) -> bool {
        self.is_head
    }

    #[must_use]
    pub fn is_merge(&self) -> bool {
        self.bellow.is_some() && !self.children.is_empty()
    }
    #[must_use]
    pub fn is_commit_link(&self) -> bool {
        self.is_commit_link
    }

    #[must_use]
    pub fn references(&self) -> &Vec<GitRef> {
        &self.references
    }
    #[must_use]
    pub fn short_id(&self) -> &String {
        &self.short_id
    }
    #[must_use]
    pub fn subject(&self) -> &String {
        &self.subject
    }
}

const REV_FORMAT: &str =
    "--format=%x1f%H%x1f%h%x1f%P%x1f%D%x1f%aN%x1f%aE%x1f%aI%x1f%ad%x1f%cN%x1f%cE%x1f%cI%x1f%cd%x1f%s%x1f%b%x1e";

impl Commit {
    #[must_use]
    pub fn new(data: &str, is_commit_link: bool) -> Self {
        let mut split = data.split('\x1f');
        split.next(); // skip commit: XXXX line
        let id = Oid {
            0: next_string!(split),
        };

        let short_id = next_string!(split);
        let mut parents_record: Vec<&str> =
            split.next().expect("Parse parents").split(' ').collect();
        if parents_record.len() == 1 && parents_record[0].is_empty() {
            parents_record = vec![];
        }
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
            if s == "HEAD" {
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
            } else if s.is_empty() {
                // do nothing
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
                icon = (*c).to_string();
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
            is_head,
            is_merge,
            branches,
            references,
            tags,
        }
    }
}

/// Return commit count with `--first-parent`
///
/// # Errors
///
/// Returns a [`PosixError`] if `working_dir` does not exist or `rev_range` is invalid.
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

/// Return specified amount of commits for a `rev_range`.
///
/// # Errors
///
/// Returns a [`PosixError`] if `working_dir` does not exist, `rev_range` is invalid or `max` &
/// `skip` combination is `>` commit length with `--first-parent`.
pub fn commits_for_range<T: AsRef<str>>(
    working_dir: &str,
    rev_range: &str,
    paths: &[T],
    skip: Option<usize>,
    max: Option<usize>,
) -> Result<Vec<Commit>, PosixError> {
    let mut args = vec!["--date=human", "--first-parent", REV_FORMAT];

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
    // let mut fork_point = ForkPointCalculation::Done(false);
    for data in lines {
        if data.is_empty() {
            break;
        }
        let commit = Commit::new(data, false);
        // if commit.is_merge {
        //     fork_point = ForkPointCalculation::Needed;
        // }
        result.push(commit);
    }
    Ok(result)
}

#[must_use]
pub fn child_history(working_dir: &str, commit: &Commit) -> Vec<Commit> {
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
    let paths: &[&str] = &[];
    #[allow(clippy::expect_fun_call)]
    let mut result = commits_for_range(working_dir, revision.as_str(), paths, None, None)
        .expect(&format!("Expected child commits for range {}", revision));
    #[allow(clippy::expect_fun_call)]
    let end_commit = result
        .last()
        .expect(&format!("No child commits for range {}", revision));
    if end.is_some()
        && end_commit.bellow.is_some()
        && end_commit.bellow.as_ref().expect("Expected merge commit") != bellow
    {
        let link = to_commit(
            working_dir,
            end_commit.bellow.as_ref().expect("Expected merge commit"),
            true,
        );

        result.push(link);
    }

    result
}

fn to_commit(working_dir: &str, oid: &Oid, is_commit_link: bool) -> Commit {
    let output = git_cmd_out(
        working_dir,
        vec!["rev-list", "--date=human", REV_FORMAT, "-1", &oid.0],
    );
    let tmp = String::from_utf8(output.unwrap().stdout);
    let lines: Vec<&str> = tmp.as_ref().expect("Valid UTF-8").lines().collect();
    // XXX FIXME lines? really?
    assert!(lines.len() >= 2, "Did not got enough data for {}", oid);
    Commit::new(lines.get(1).unwrap(), is_commit_link)
}

/// Return the mergebase for two commit ids
///
/// # Errors
/// Return [`PosixError`] when `merge-base` command fails. Should never happen.
pub fn merge_base(working_dir: &str, p1: &Oid, p2: &Oid) -> Result<Option<Oid>, PosixError> {
    let output = git_wrapper::git_cmd_out(working_dir, vec!["merge-base", &p1.0, &p2.0]);
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
