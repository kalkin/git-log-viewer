use crossterm::style::{style, Attribute, ContentStyle, StyledContent};
use gsi::SubtreeConfig;
use url::Url;

use crate::actors::fork_point::ForkPointCalculation;
use crate::commit::{Commit, GitRef, Oid};
use crate::default_styles::{DATE_STYLE, ID_STYLE, MOD_STYLE, NAME_STYLE, REF_STYLE};
use crate::ui::base::StyledLine;
use git_wrapper::Remote;
use subject_classifier::{Subject, SubtreeOperation};
use unicode_truncate::UnicodeTruncateStr;
use unicode_width::UnicodeWidthStr;

#[allow(clippy::module_name_repetitions, dead_code)]
pub struct HistoryEntry {
    commit: Commit,
    folded: bool,
    level: u8,
    remotes: Vec<Remote>,
    subject_text: String,
    subject_struct: Subject,
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
        remotes: &[Remote],
    ) -> Self {
        let subject_struct = Subject::from(commit.subject().as_str());
        let subject_text = subject_struct.description().to_string();

        // let special_subject = are_we_special(&commit);
        let remotes = if commit.references().is_empty() {
            vec![]
        } else {
            let mut result = vec![];
            for remote in remotes {
                for git_ref in commit.references() {
                    if git_ref.to_string().starts_with(&remote.name) {
                        result.push(remote.clone());
                        break;
                    }
                }
            }
            result
        };

        HistoryEntry {
            commit,
            folded: true,
            level,
            remotes,
            subject_text,
            subject_struct,
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
        style(self.subject_struct.icon().to_string())
    }

    fn render_graph(&self) -> StyledContent<String> {
        let mut text = "".to_string();
        for _ in 0..self.level {
            text.push_str("│ ");
        }

        if self.commit.bellow().is_none() {
            text.push('◉');
        } else if self.is_commit_link() {
            text.push('⭞');
        } else {
            text.push('●');
        }

        if self.has_children() {
            if self.is_subtree_import() || self.is_subtree_update() {
                if self.is_fork_point() {
                    text.push_str("⇤┤");
                } else {
                    text.push_str("⇤╮");
                }
            } else if self.is_fork_point() {
                text.push_str("─┤");
            } else {
                text.push_str("─┐");
            }
        } else if self.is_fork_point() {
            text.push_str("─┘");
        }
        style(text)
    }
    fn render_modules(&self, max_len: usize) -> Option<StyledContent<String>> {
        if self.subtrees.is_empty() {
            None
        } else {
            let mut text = ":".to_string();
            let subtree_modules: Vec<String> =
                self.subtrees.iter().map(SubtreeConfig::id).collect();
            text.push_str(&subtree_modules.join(" :"));
            if text.width() > max_len {
                match subtree_modules.len() {
                    1 => {
                        text = text.unicode_truncate(max_len - 1).0.to_string();
                        text.push('…');
                    }
                    x => text = format!("({} strees)", x),
                }
            }
            Some(StyledContent::new(*MOD_STYLE, text))
        }
    }

    fn shorten_references(remotes: &[Remote], references: &[GitRef]) -> Vec<String> {
        let mut result = vec![];
        if !references.is_empty() {
            if remotes.is_empty() {
                for r in references {
                    result.push(r.to_string());
                }
            } else {
                let mut mut_refs = references.to_vec();
                let mut tmp_result = vec![];
                for remote in remotes {
                    let mut remote_branches = vec![];
                    if mut_refs.is_empty() {
                        break;
                    }
                    for git_ref in references {
                        if git_ref.to_string().starts_with(&remote.name) {
                            remote_branches.push(git_ref);
                            mut_refs.retain(|x| x != git_ref);
                        }
                    }
                    if !remote_branches.is_empty() {
                        if remote_branches.len() == 1 {
                            result.push(remote_branches[0].to_string());
                        } else {
                            let prefix_len = remote.name.len() + 1;
                            let mut text = remote.name.to_string();
                            text.push('/');
                            text.push('{');
                            text.push_str(
                                &remote_branches
                                    .iter()
                                    .map(|r| r.to_string().split_off(prefix_len))
                                    .collect::<Vec<_>>()
                                    .join(","),
                            );
                            text.push('}');
                            tmp_result.push(text);
                        }
                    }
                }
                result.extend(mut_refs.iter().map(std::string::ToString::to_string));
                result.extend(tmp_result);
            }
        }
        result
    }

    fn format_scope(scope: &str) -> StyledContent<String> {
        let mut text = "(".to_string();
        text.push_str(scope);
        text.push(')');
        StyledContent::new(ContentStyle::default(), text)
    }

    fn render_references(&self) -> Vec<StyledContent<String>> {
        let mut result = vec![];
        for r in HistoryEntry::shorten_references(&self.remotes, self.commit.references()) {
            let separator = style(" ".to_string());
            result.push(separator);

            let text = format!("«{}»", r);
            let sc = StyledContent::new(*REF_STYLE, text);
            result.push(sc);
        }
        result
    }
    fn render_subject(&self) -> Vec<StyledContent<String>> {
        let mut buf = vec![];
        let separator = style(" ".to_string());
        if let Some(modules) = self.render_modules(32) {
            buf.push(modules);
            buf.push(separator.clone());
        }
        match &self.subject_struct {
            Subject::ConventionalCommit {
                scope, description, ..
            } => {
                if let Some(scope) = scope {
                    buf.push(HistoryEntry::format_scope(scope));
                    buf.push(separator);
                }
                buf.push(StyledContent::new(
                    ContentStyle::default(),
                    description.clone(),
                ));
            }
            Subject::Release { description, .. }
            | Subject::Fixup(description)
            | Subject::Remove(description)
            | Subject::Rename(description)
            | Subject::Revert(description)
            | Subject::Simple(description) => buf.push(StyledContent::new(
                ContentStyle::default(),
                description.clone(),
            )),
            Subject::PullRequest { .. } => buf.push(style(self.subject_text.clone())),
            Subject::SubtreeCommit { operation, .. } => {
                let mut bold_style = ContentStyle::default();
                bold_style.attributes.set(Attribute::Bold);
                let (text, git_ref) = match operation {
                    SubtreeOperation::Import { git_ref, .. } => ("Import from ", git_ref),
                    SubtreeOperation::Split { git_ref, .. } => ("Split into commit ", git_ref),
                    SubtreeOperation::Update { git_ref, .. } => ("Update to ", git_ref),
                };
                buf.push(style(text.to_string()));
                let sc = StyledContent::new(bold_style, git_ref.clone());
                buf.push(sc);
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
    fn is_subtree_import(&self) -> bool {
        matches!(
            &self.subject_struct,
            Subject::SubtreeCommit {
                operation: SubtreeOperation::Import { .. },
                ..
            }
        )
    }

    fn is_subtree_update(&self) -> bool {
        matches!(
            &self.subject_struct,
            Subject::SubtreeCommit {
                operation: SubtreeOperation::Update { .. },
                ..
            }
        )
    }
}
// Public interface
impl HistoryEntry {
    pub fn set_subject(&mut self, subject: String) {
        self.subject_text = subject;
    }

    pub fn set_fork_point(&mut self, t: bool) {
        self.fork_point = ForkPointCalculation::Done(t);
    }

    #[must_use]
    pub fn special(&self) -> &Subject {
        &self.subject_struct
    }

    #[must_use]
    pub fn body(&self) -> &String {
        self.commit.body()
    }

    #[must_use]
    pub fn subject(&self) -> &String {
        &self.subject_text
    }

    #[must_use]
    pub fn original_subject(&self) -> &String {
        self.commit.subject()
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
            &self.subject_text,
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
