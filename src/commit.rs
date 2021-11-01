use crate::ui::base::search::Needle;

use getset::Getters;
use git_wrapper::git_cmd_out;
use posix_errors::PosixError;
use std::fmt::{Debug, Display, Formatter};

macro_rules! next_string {
    ($split:expr) => {
        $split.next().expect("Another split").to_string()
    };
}

#[derive(Clone, Eq, PartialEq)]
pub struct Oid(pub String);

impl Display for Oid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Debug for Oid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut text = self.0.clone();
        text.truncate(8);
        f.write_str(&text)
    }
}

#[derive(Eq, PartialEq, Clone)]
pub struct GitRef(pub String);

impl Display for GitRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Getters, Clone)]
pub struct Commit {
    #[getset(get = "pub")]
    id: Oid,
    #[getset(get = "pub")]
    short_id: String,
    #[getset(get = "pub")]
    author_name: String,
    #[getset(get = "pub")]
    author_email: String,
    #[getset(get = "pub")]
    author_date: String,
    #[getset(get = "pub")]
    author_rel_date: String,
    #[getset(get = "pub")]
    committer_name: String,
    #[getset(get = "pub")]
    committer_email: String,
    #[getset(get = "pub")]
    committer_date: String,
    #[getset(get = "pub")]
    committer_rel_date: String,
    #[getset(get = "pub")]
    subject: String,
    #[getset(get = "pub")]
    body: String,

    #[getset(get = "pub")]
    bellow: Option<Oid>,
    #[getset(get = "pub")]
    children: Vec<Oid>,
    #[getset(get = "pub")]
    is_commit_link: bool,
    is_head: bool,
    is_merge: bool,
    branches: Vec<GitRef>,
    #[getset(get = "pub")]
    references: Vec<GitRef>,
    tags: Vec<GitRef>,
}

impl Commit {
    #[must_use]
    pub fn is_merge(&self) -> bool {
        self.bellow.is_some() && !self.children.is_empty()
    }

    pub fn matches(&self, needle: &Needle) -> bool {
        let candidates = vec![
            self.author_name(),
            self.short_id(),
            &self.id().0,
            self.author_name(),
            self.author_email(),
            self.committer_name(),
            self.committer_email(),
            &self.subject,
        ];
        for text in candidates {
            if text.contains(needle.text()) {
                return true;
            }
        }
        false
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
                is_head = true;
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

        Commit {
            id,
            short_id,
            author_name,
            author_email,
            author_date,
            author_rel_date,
            committer_name,
            committer_email,
            committer_date,
            committer_rel_date,
            subject,
            body,
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
    paths: &[String],
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
pub fn child_history(working_dir: &str, commit: &Commit, paths: &[String]) -> Vec<Commit> {
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

#[cfg(test)]
mod test {
    use crate::commit::commits_for_range;

    #[test]
    fn initial_commit() {
        let working_dir = git_wrapper::top_level().unwrap();
        let paths: &[&str] = &[];
        let result = commits_for_range(&working_dir, "a17989470af", paths, None, None).unwrap();
        assert_eq!(result.len(), 1);
        let commit = &result[0];
        assert_eq!(commit.children.len(), 0);
        assert_eq!(commit.bellow, None);
        assert!(!commit.is_merge);
    }
}
