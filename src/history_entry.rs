use crossterm::style::{style, Attribute, ContentStyle, StyledContent};
use gsi::{SubtreeConfig, SubtreeOperation};
use url::Url;

use crate::actors::fork_point::ForkPointCalculation;
use crate::commit::{Commit, Oid};
use crate::default_styles::{DATE_STYLE, ID_STYLE, MOD_STYLE, NAME_STYLE, REF_STYLE};
use crate::ui::base::StyledLine;
use lazy_static::lazy_static;
use regex::Regex;
use unicode_width::UnicodeWidthStr;

#[derive(Eq, PartialEq)]
pub enum SpecialSubject {
    PrMerge(String),
    None,
}

#[allow(clippy::module_name_repetitions, dead_code)]
pub struct HistoryEntry {
    commit: Commit,
    folded: bool,
    level: u8,
    subtree_operation: SubtreeOperation,
    subject_module: Option<String>,
    subject: String,
    special_subject: SpecialSubject,
    pub subtrees: Vec<SubtreeConfig>,
    repo_url: Option<Url>,
    fork_point: ForkPointCalculation,
    working_dir: String,
}

impl HistoryEntry {
    #[must_use]
    pub fn new(
        working_dir: String,
        commit: Commit,
        level: u8,
        repo_url: Option<Url>,
        fork_point: ForkPointCalculation,
    ) -> Self {
        let subtree_operation = SubtreeOperation::from(commit.subject());

        let (subject_module, short_subject) = split_subject(&commit.subject());
        let subject = short_subject.unwrap_or_else(|| commit.subject().clone());

        let special_subject = are_we_special(&commit);

        HistoryEntry {
            commit,
            folded: true,
            level,
            subject,
            special_subject,
            subject_module,
            subtree_operation,
            subtrees: vec![],
            repo_url,
            fork_point,
            working_dir,
        }
    }
}
// Rendering operations
impl HistoryEntry {
    fn render_id(&self) -> StyledContent<String> {
        let id = self.commit.short_id();
        StyledContent::new(*ID_STYLE, id.clone())
    }

    fn render_date(&self) -> StyledContent<String> {
        let date = self.author_rel_date();
        StyledContent::new(*DATE_STYLE, date.clone())
    }

    fn render_name(&self) -> StyledContent<String> {
        let name = self.commit.author_name();
        StyledContent::new(*NAME_STYLE, name.clone())
    }

    fn render_icon(&self) -> StyledContent<String> {
        style(self.commit.icon().clone())
    }

    fn render_graph(&self) -> StyledContent<String> {
        let mut text = "".to_string();
        for _ in 0..self.level {
            text.push_str("│ ")
        }

        if self.commit.bellow().is_none() {
            text.push('◉')
        } else if self.is_commit_link() {
            text.push('⭞')
        } else {
            text.push('●')
        }

        if self.has_children() {
            if self.subtree_operation.is_import() || self.subtree_operation.is_update() {
                if self.is_fork_point() {
                    text.push_str("⇤┤");
                } else {
                    text.push_str("⇤╮");
                }
            } else if self.is_fork_point() {
                text.push_str("─┤");
            } else {
                text.push_str("─┐")
            }
        } else if self.is_fork_point() {
            text.push_str("─┘")
        }
        style(text)
    }
    fn render_modules(&self, max_len: usize) -> Option<StyledContent<String>> {
        let mut text;
        match (!self.subtrees.is_empty(), self.subject_module.is_some()) {
            (true, _) => {
                text = ":".to_string();
                let subtree_modules: Vec<String> =
                    self.subtrees.iter().map(SubtreeConfig::id).collect();
                text.push_str(&subtree_modules.join(" :"));
                if text.width() > max_len {
                    text = format!("({} modules)", subtree_modules.len());
                }
            }
            (false, true) => text = self.subject_module.as_ref().unwrap().clone(),
            (false, false) => return None,
        };
        Some(StyledContent::new(*MOD_STYLE, text))
    }

    fn render_references(&self) -> Vec<StyledContent<String>> {
        let mut result = vec![];
        for r in self.commit.references() {
            let separator = style(" ".to_string());
            result.push(separator);

            let text = format!("«{}»", r);
            let sc = StyledContent::new(*REF_STYLE, text);
            result.push(sc);
        }
        result
    }
    fn render_subject(&self) -> Vec<StyledContent<String>> {
        let mut bold_style = ContentStyle::default();
        bold_style.attributes.set(Attribute::Bold);
        let mut buf = vec![];
        match &self.subtree_operation {
            SubtreeOperation::Update { subtree, git_ref } => {
                buf.push(StyledContent::new(*MOD_STYLE, subtree.to_owned()));
                buf.push(style(" Update to ".to_string()));
                let sc = StyledContent::new(bold_style, git_ref.to_owned());
                buf.push(sc);
            }
            SubtreeOperation::Split { subtree, git_ref } => {
                buf.push(StyledContent::new(*MOD_STYLE, subtree.to_owned()));
                buf.push(style(" Split into commit ".to_string()));
                let sc = StyledContent::new(bold_style, git_ref.to_owned());
                buf.push(sc);
            }
            SubtreeOperation::Import { subtree, git_ref } => {
                buf.push(StyledContent::new(*MOD_STYLE, subtree.to_owned()));
                buf.push(style(" Import from ".to_string()));
                let sc = StyledContent::new(bold_style, git_ref.to_owned());
                buf.push(sc);
            }
            _ => {
                if let Some(modules) = self.render_modules(32) {
                    buf.push(modules);
                }
                let separator = style(" ".to_string());
                buf.push(separator);
                let text = self.subject.clone();
                buf.push(style(text));
            }
        }

        buf
    }

    pub fn render(&mut self, selected: bool) -> StyledLine<String> {
        let separator = style(" ".to_string());
        let mut result: StyledLine<String> = vec![
            self.render_id(),
            separator.clone(),
            self.render_date(),
            separator.clone(),
            self.render_name(),
            separator.clone(),
            self.render_icon(),
            separator.clone(),
            self.render_graph(),
        ];
        let references = self.render_references();
        if !references.is_empty() {
            result.extend(references);
        }
        result.push(separator);
        result.extend(self.render_subject());

        if selected {
            for part in &mut result {
                part.style_mut().attributes.set(Attribute::Reverse);
            }
        };
        result
    }
}
// Public interface
impl HistoryEntry {
    pub fn set_subject(&mut self, subject: String) {
        self.subject = subject
    }

    pub fn set_fork_point(&mut self, t: bool) {
        self.fork_point = ForkPointCalculation::Done(t);
    }

    #[must_use]
    pub fn special(&self) -> &SpecialSubject {
        &self.special_subject
    }

    #[must_use]
    pub fn body(&self) -> &String {
        &self.commit.body()
    }

    #[must_use]
    pub fn subject(&self) -> &String {
        &self.subject
    }

    #[must_use]
    pub fn original_subject(&self) -> &String {
        &self.commit.subject()
    }

    #[must_use]
    pub fn commit(&self) -> &Commit {
        &self.commit
    }

    #[must_use]
    pub fn id(&self) -> &Oid {
        self.commit.id()
    }

    #[must_use]
    pub fn short_id(&self) -> &String {
        self.commit.short_id()
    }

    #[must_use]
    pub fn author_rel_date(&self) -> &String {
        self.commit.author_rel_date()
    }

    #[must_use]
    pub fn author_date(&self) -> &String {
        self.commit.author_date()
    }

    #[must_use]
    pub fn committer_date(&self) -> &String {
        self.commit.committer_date()
    }

    #[must_use]
    pub fn author_name(&self) -> &String {
        self.commit.author_name()
    }

    #[must_use]
    pub fn committer_name(&self) -> &String {
        self.commit.committer_name()
    }

    #[must_use]
    pub fn is_fork_point(&self) -> bool {
        match self.fork_point {
            ForkPointCalculation::Done(t) => t,
            ForkPointCalculation::InProgress => false,
        }
    }

    pub fn folded(&mut self, t: bool) {
        self.folded = t;
    }

    #[must_use]
    pub fn is_folded(&self) -> bool {
        self.folded
    }

    #[must_use]
    pub fn is_foldable(&self) -> bool {
        self.commit.is_merge()
    }

    #[must_use]
    pub fn has_children(&self) -> bool {
        self.commit.is_merge()
    }

    #[must_use]
    pub fn level(&self) -> u8 {
        self.level
    }

    #[must_use]
    pub fn is_commit_link(&self) -> bool {
        self.commit.is_commit_link()
    }

    /// Check if string is contained any where in commit data
    #[must_use]
    #[allow(dead_code)]
    pub fn search_matches(&self, needle: &str, ignore_case: bool) -> bool {
        let mut candidates = vec![
            self.commit.author_name(),
            self.commit.short_id(),
            &self.commit.id().0,
            self.commit.author_name(),
            self.commit.author_email(),
            self.commit.committer_name(),
            self.commit.committer_email(),
            &self.subject,
        ];

        let x: Vec<String> = self.subtrees.iter().map(SubtreeConfig::id).collect();
        candidates.extend(&x);

        for r in self.commit.references().iter() {
            candidates.push(&r.0);
        }

        for cand in candidates {
            if ignore_case {
                if cand.to_lowercase().contains(&needle.to_lowercase()) {
                    return true;
                }
            } else {
                return cand.contains(needle);
            }
        }
        false
    }

    #[must_use]
    pub fn subtrees(&self) -> &Vec<SubtreeConfig> {
        &self.subtrees
    }
    #[must_use]
    pub fn url(&self) -> Option<Url> {
        if let Some(module) = self.subtrees.first() {
            if let Some(v) = module.upstream().or_else(|| module.origin()) {
                if let Ok(u) = Url::parse(&v) {
                    return Some(u);
                }
            }
        }
        self.repo_url.clone()
    }

    #[must_use]
    pub fn working_dir(&self) -> &String {
        &self.working_dir
    }
}

lazy_static! {
    static ref SPLIT_SUBJ_REGEX: Regex = regex!(r"^\w+\((.+)\):\s?.+");
    static ref GH_SPECIAL_REGEX: Regex =
        regex!(r"^Merge (?:remote-tracking branch '.+/pr/(\d+)'|pull request #(\d+) from .+)$");
}
#[must_use]
pub fn split_subject(subject: &str) -> (Option<String>, Option<String>) {
    let mut subject_module = None;
    let mut short_subject = None;
    if subject.contains("):") {
        if let Some(caps) = SPLIT_SUBJ_REGEX.captures(&subject) {
            let x = caps.get(1).expect("Expected 1 capture group");
            subject_module = Some(x.as_str().to_string());
            let mut f = subject.to_string();
            f.truncate(x.start() - 1);
            f.push_str(&subject.to_string().split_off(x.end() + 1));
            short_subject = Some(f);
        }
    }
    (subject_module, short_subject)
}

fn are_we_special(commit: &Commit) -> SpecialSubject {
    let mut special_subject = SpecialSubject::None;
    if commit.is_merge() && GH_SPECIAL_REGEX.is_match(&commit.subject()) {
        if let Some(caps) = GH_SPECIAL_REGEX.captures(&commit.subject()) {
            let pr_id = if let Some(n) = caps.get(1) {
                n.as_str().to_string()
            } else if let Some(n) = caps.get(2) {
                n.as_str().to_string()
            } else {
                panic!("Failed to ideintify pr number {:?}", caps);
            };

            special_subject = SpecialSubject::PrMerge(pr_id);
        }
    }

    special_subject
}
