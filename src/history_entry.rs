// Copyright (C) 2021  Bahtiar `kalkin-` Gadimov <bahtiar@gadimov.de>
//
// This file is part of git-log-viewer
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use url::Url;

use crossterm::style::{style, Attribute, ContentStyle, StyledContent};
use getset::{CopyGetters, Getters, Setters};
use git_stree::SubtreeConfig;

use crate::actors::fork_point::ForkPointCalculation;
use crate::commit::{parse_remote_url, Commit, GitRef, Oid};
use crate::default_styles::{DATE_STYLE, ID_STYLE, MOD_STYLE, NAME_STYLE, REF_STYLE};
use crate::ui::base::StyledLine;
use git_wrapper::Remote;
use lazy_static::lazy_static;
use subject_classifier::{Subject, SubtreeOperation};
use unicode_truncate::UnicodeTruncateStr;
use unicode_width::UnicodeWidthStr;

lazy_static! {
    static ref TIME_SPLIT_REGEX: regex::Regex =
        regex::Regex::new(r#".+{8,} \d\d:\d\d$"#).expect("Valid RegEx");
}

#[derive(CopyGetters, Getters, Setters)]
pub struct HistoryEntry {
    #[getset(get = "pub")]
    commit: Commit,
    folded: bool,
    #[getset(get_copy = "pub")]
    level: u8,
    remotes: Vec<Remote>,
    subject_text: String,
    subject_struct: Subject,
    #[getset(get = "pub", set = "pub")]
    subtrees: Vec<SubtreeConfig>,
    #[getset(get = "pub", set = "pub")]
    forge_url: Option<Url>,
    fork_point: ForkPointCalculation,
}

impl HistoryEntry {
    #[must_use]
    pub fn new(
        commit: Commit,
        level: u8,
        forge_url: Option<Url>,
        fork_point: ForkPointCalculation,
        repo_remotes: &[Remote],
    ) -> Self {
        let subject_struct = Subject::from(commit.subject().as_str());
        let subject_text = subject_struct.description().to_owned();

        // let special_subject = are_we_special(&commit);
        let remotes = if commit.references().is_empty() {
            vec![]
        } else {
            let mut result = vec![];
            for remote in repo_remotes {
                for git_ref in commit.references() {
                    if git_ref.to_string().starts_with(&remote.name) {
                        result.push(remote.clone());
                        break;
                    }
                }
            }
            result
        };

        Self {
            commit,
            folded: true,
            level,
            remotes,
            subject_text,
            subject_struct,
            subtrees: vec![],
            forge_url,
            fork_point,
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
        if TIME_SPLIT_REGEX.is_match(date) {
            StyledContent::new(*DATE_STYLE, date[0..date.len() - 5].to_owned())
        } else {
            StyledContent::new(*DATE_STYLE, date.clone())
        }
    }

    fn render_name(&self) -> StyledContent<String> {
        let name = self.commit.author_name();
        StyledContent::new(*NAME_STYLE, name.clone())
    }

    fn render_icon(&self) -> StyledContent<String> {
        style(self.subject_struct.icon().to_owned())
    }

    fn render_graph(&self) -> StyledContent<String> {
        let mut text = "".to_owned();
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

        #[allow(clippy::else_if_without_else)]
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
            let mut text = ":".to_owned();
            let subtree_modules: Vec<String> =
                self.subtrees.iter().map(|e| e.id().clone()).collect();
            text.push_str(&subtree_modules.join(" :"));
            if text.width() > max_len {
                match subtree_modules.len() {
                    1 => {
                        text = text.unicode_truncate(max_len - 1).0.to_owned();
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
                            let mut text = remote.name.clone();
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
        let mut text = "(".to_owned();
        text.push_str(scope);
        text.push(')');
        StyledContent::new(ContentStyle::default(), text)
    }

    fn render_references(&self) -> Vec<StyledContent<String>> {
        let mut result = vec![];
        for r in Self::shorten_references(&self.remotes, self.commit.references()) {
            let separator = style(" ".to_owned());
            result.push(separator);

            let text = format!("«{}»", r);
            let sc = StyledContent::new(*REF_STYLE, text);
            result.push(sc);
        }
        result
    }
    fn render_subject(&self) -> Vec<StyledContent<String>> {
        let mut buf = vec![];
        let separator = style(" ".to_owned());
        if let Some(modules) = self.render_modules(32) {
            buf.push(modules);
            buf.push(separator.clone());
        }
        match &self.subject_struct {
            Subject::ConventionalCommit {
                scope, description, ..
            } => {
                if let Some(s) = scope {
                    buf.push(Self::format_scope(s));
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
                buf.push(style(text.to_owned()));
                let sc = StyledContent::new(bold_style, git_ref.clone());
                buf.push(sc);
            }
        }

        buf
    }

    pub fn render(&mut self, selected: bool) -> StyledLine<String> {
        let separator = style(" ".to_owned());
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
    const fn is_subtree_import(&self) -> bool {
        matches!(
            &self.subject_struct,
            Subject::SubtreeCommit {
                operation: SubtreeOperation::Import { .. },
                ..
            }
        )
    }

    const fn is_subtree_update(&self) -> bool {
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
    pub fn set_subject(&mut self, subject: &str) {
        self.subject_struct = Subject::from(subject);
        self.subject_text = self.subject_struct.description().to_owned();
    }

    pub fn set_fork_point(&mut self, t: bool) {
        self.fork_point = ForkPointCalculation::Done(t);
    }

    #[must_use]
    pub const fn special(&self) -> &Subject {
        &self.subject_struct
    }

    #[must_use]
    pub fn body(&self) -> &String {
        self.commit.body()
    }
    #[must_use]
    pub fn original_subject(&self) -> &String {
        self.commit.subject()
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
    pub const fn is_fork_point(&self) -> bool {
        match self.fork_point {
            ForkPointCalculation::Done(t) => t,
            ForkPointCalculation::InProgress => false,
        }
    }

    pub fn folded(&mut self, t: bool) {
        self.folded = t;
    }

    #[must_use]
    pub const fn is_folded(&self) -> bool {
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
    pub fn is_commit_link(&self) -> bool {
        *self.commit.is_commit_link()
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

        let x: Vec<String> = self.subtrees.iter().map(|e| e.id().clone()).collect();
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
    pub fn url(&self) -> Option<Url> {
        if let Some(module) = self.subtrees.first() {
            let url_option = if module.upstream().is_some() {
                module.upstream()
            } else {
                module.origin()
            };
            if let Some(v) = url_option {
                if let Some(u) = parse_remote_url(v) {
                    return Some(u);
                };
            }
        }
        self.forge_url.clone()
    }
}
