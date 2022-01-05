use crate::ui::base::search::Needle;

use getset::Getters;
use git_wrapper::Repository;
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
    #[allow(dead_code)]
    is_head: bool,
    #[allow(dead_code)]
    is_merge: bool,
    #[allow(dead_code)]
    branches: Vec<GitRef>,
    #[getset(get = "pub")]
    references: Vec<GitRef>,
    #[allow(dead_code)]
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
    repo: &Repository,
    rev_range: &str,
    paths: &[String],
) -> Result<usize, PosixError> {
    let mut git = repo.git();
    git.args(vec!["rev-list", "--first-parent", "--count", rev_range]);
    if !paths.is_empty() {
        git.arg("--");
        for p in paths {
            git.arg(p);
        }
    }
    let proc = git.output().expect("Failed to run rev-list");

    if proc.status.success() {
        let text = String::from_utf8_lossy(&proc.stdout).trim_end().to_string();
        return Ok(text
            .parse::<usize>()
            .expect("Failed to parse commit length"));
    }

    Err(PosixError::from(proc))
}

pub fn commits_for_range<T: AsRef<str>>(
    repo: &Repository,
    rev_range: &str,
    paths: &[T],
    skip: Option<usize>,
    max: Option<usize>,
) -> Vec<Commit> {
    let mut cmd = repo.git();
    cmd.arg("rev-list")
        .args(vec!["--date=human", "--first-parent", REV_FORMAT]);

    let tmp;
    if let Some(val) = skip {
        tmp = format!("--skip={}", val);
        cmd.arg(&tmp);
    }

    let tmp2;
    if let Some(val) = max {
        tmp2 = format!("--max-count={}", val);
        cmd.arg(&tmp2);
    }

    cmd.arg(rev_range);

    if !paths.is_empty() {
        cmd.arg("--");
        for p in paths {
            cmd.arg(p.as_ref());
        }
    }

    let proc = cmd.output().expect("Failed to run git-rev-list(1)");
    if proc.status.success() {
        let output = String::from_utf8_lossy(&proc.stdout);
        let lines = output.split('\u{1e}');
        let mut result: Vec<Commit> = Vec::new();
        for data in lines {
            if data.is_empty() || data == "\n" {
                break;
            }
            result.push(Commit::new(data, false));
        }
        return result;
    }
    eprintln!(
        "Failed to find commits for range({}), with skip({:?}) / max({:?}) & path({})",
        rev_range,
        skip,
        max,
        paths.is_empty()
    );
    return vec![];
}

#[must_use]
pub fn child_history(repo: &Repository, commit: &Commit, paths: &[String]) -> Vec<Commit> {
    let bellow = commit.bellow.as_ref().expect("Expected merge commit");
    let first_child = commit.children.get(0).expect("Expected merge commit");
    let end = repo
        .merge_base(&[&bellow.0, &first_child.0])
        .expect("merge base shouldn't fail");

    let revision;
    if let Some(v) = &end {
        if v == &first_child.0 {
            revision = first_child.0.clone();
        } else {
            revision = format!("{}..{}", v, first_child.0);
        }
    } else {
        revision = first_child.0.clone();
    }
    let mut result = commits_for_range(repo, revision.as_str(), paths, None, None);

    let end_commit = result
        .last()
        .unwrap_or_else(|| panic!("No child commits for range {}", revision));
    if end.is_some()
        && end_commit.bellow.is_some()
        && end_commit.bellow.as_ref().expect("Expected merge commit") != bellow
    {
        let link = to_commit(
            repo,
            end_commit.bellow.as_ref().expect("Expected merge commit"),
            true,
        );

        result.push(link);
    }

    result
}

fn to_commit(repo: &Repository, oid: &Oid, is_commit_link: bool) -> Commit {
    let mut cmd = repo.git();
    cmd.args(["rev-list", "--date=human", REV_FORMAT, "-1", &oid.0]);
    let proc = cmd.output().expect("Failed to run git-rev-list(1)");
    if proc.status.success() {
        let tmp = String::from_utf8_lossy(&proc.stdout);
        let lines: Vec<&str> = tmp.lines().collect();
        // XXX FIXME lines? really?
        assert!(lines.len() >= 2, "Did not got enough data for {}", oid);
        Commit::new(lines.get(1).unwrap(), is_commit_link)
    } else {
        panic!("Failed to get data for commit {}", oid);
    }
}

#[cfg(test)]
mod test {
    use crate::commit::commits_for_range;
    use git_wrapper::Repository;

    #[test]
    fn initial_commit() {
        let repo = Repository::default().unwrap();
        let paths: &[&str] = &[];
        let result = commits_for_range(&repo, "a17989470af", paths, None, None);
        assert_eq!(result.len(), 1);
        let commit = &result[0];
        assert_eq!(commit.children.len(), 0);
        assert_eq!(commit.bellow, None);
        assert!(!commit.is_merge);
    }
}
