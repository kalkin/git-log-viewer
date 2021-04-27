use cursive::theme::{Effect, Style};
use cursive::utils::span::SpannedString;
use regex::Regex;
use url::Url;

use git_subtrees_improved::SubtreeConfig;

use crate::core::Commit;
use crate::search::SearchState;
use crate::style::{date_style, id_style, mod_style, name_style, ref_style, DEFAULT_STYLE};
use std::borrow::BorrowMut;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

macro_rules! search_if_needed {
    ($text:expr,$style:expr,$optional_search_state:expr) => {
        if let Some(search_state) = $optional_search_state {
            HistoryEntry::highlight_search($style, &$text, search_state)
        } else {
            SpannedString::styled($text, $style)
        }
    };
}
#[derive(Eq, PartialEq)]
pub enum SubtreeOperation {
    Update,
    Import,
    Split,
    None,
}

#[derive(Eq, PartialEq)]
pub enum SpecialSubject {
    PrMerge(String),
    None,
}

pub struct HistoryEntry {
    commit: Commit,
    folded: bool,
    level: u8,
    subtree_operation: SubtreeOperation,
    subject_module: Option<String>,
    subject: String,
    special_subject: SpecialSubject,
    selected: bool,
    pub subtrees: Vec<SubtreeConfig>,
    repo_url: Option<Url>,
}

impl HistoryEntry {
    pub(crate) fn subtrees(&self) -> &Vec<SubtreeConfig> {
        &self.subtrees
    }
    pub(crate) fn url(&self) -> Option<Url> {
        if self.subtrees.len() == 1 {
            let module = self.subtrees.first().unwrap();
            if let Some(v) = module.upstream().or(module.origin()) {
                if let Ok(u) = Url::parse(&v) {
                    return Some(u);
                }
            }
        }
        self.repo_url.clone()
    }
}

pub struct WidthConfig {
    pub max_author: usize,
    pub max_date: usize,
    pub max_modules: usize,
}

struct SearchMatch {
    start: usize,
    end: usize,
}

impl HistoryEntry {
    pub fn new(commit: Commit, level: u8, repo_url: Option<Url>) -> Self {
        let mut subtree_operation = SubtreeOperation::None;
        if commit.subject().starts_with("Update :") {
            subtree_operation = SubtreeOperation::Update
        } else if commit.subject().starts_with("Import :") {
            subtree_operation = SubtreeOperation::Import
        } else if commit.subject().starts_with("Split '") {
            subtree_operation = SubtreeOperation::Split
        }

        let (subject_module, short_subject) = split_subject(&commit.subject());
        let subject = short_subject.unwrap_or_else(|| commit.subject().clone());

        let special_subject = HistoryEntry::are_we_special(&commit);

        HistoryEntry {
            commit,
            folded: true,
            level,
            subject,
            special_subject,
            selected: false,
            subject_module,
            subtree_operation,
            subtrees: vec![],
            repo_url,
        }
    }

    pub fn set_subject(&mut self, subject: String) {
        self.subject = subject
    }

    pub fn special(&self) -> &SpecialSubject {
        &self.special_subject
    }

    fn are_we_special(commit: &Commit) -> SpecialSubject {
        let mut special_subject = SpecialSubject::None;
        let local_gh_merge = regex!(r"^Merge remote-tracking branch '.+/pr/(\d+)'$");
        if let Some(caps) = local_gh_merge.captures(&commit.subject()) {
            special_subject = SpecialSubject::PrMerge(caps.get(1).unwrap().as_str().to_string())
        }

        let online_gh_merge = regex!(r"^Merge pull request #(\d+) from .+$");
        if let Some(caps) = online_gh_merge.captures(&commit.subject()) {
            special_subject = SpecialSubject::PrMerge(caps.get(1).unwrap().as_str().to_string())
        }
        special_subject
    }

    fn name_span(
        &self,
        search_state: Option<&SearchState>,
        max_len: usize,
    ) -> SpannedString<Style> {
        let style = name_style(&self.default_style());
        let text = adjust_string(self.commit.author_name(), max_len);
        search_if_needed!(text, style, search_state)
    }

    pub fn commit(&self) -> &Commit {
        &self.commit
    }

    pub fn commit_mut(&mut self) -> &mut Commit {
        self.commit.borrow_mut()
    }

    pub fn folded(&mut self, t: bool) {
        self.folded = t;
    }

    pub fn is_folded(&self) -> bool {
        self.folded
    }

    pub fn is_merge(&self) -> bool {
        self.commit.is_merge()
    }

    pub fn level(&self) -> u8 {
        self.level
    }

    pub fn is_commit_link(&self) -> bool {
        self.commit.is_commit_link()
    }

    pub fn selected(&mut self, t: bool) {
        self.selected = t;
    }

    fn default_style(&self) -> Style {
        if self.selected {
            let mut style = *DEFAULT_STYLE;
            style.effects |= Effect::Reverse;
            style
        } else {
            *DEFAULT_STYLE
        }
    }

    fn search_text(haystack: &str, needle: &str) -> Vec<SearchMatch> {
        let mut result = Vec::new();
        let indices = haystack.match_indices(needle);
        for (i, s) in indices {
            result.push(SearchMatch {
                start: i,
                end: i + s.len(),
            })
        }

        result
    }

    /// Check if string is contained any where in commit data
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

        let x: Vec<String> = self.subtrees.iter().map(|m| m.id()).collect();
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

    fn date_span(&self, max_len: usize) -> SpannedString<Style> {
        let style = date_style(&self.default_style());
        let text = adjust_string(self.commit.author_rel_date(), max_len);
        let mut result = SpannedString::new();
        result.append_styled(text, style);
        result
    }

    fn id_span(&self, search_state: Option<&SearchState>) -> SpannedString<Style> {
        let style = id_style(&self.default_style());
        let text = self.commit.short_id();
        search_if_needed!(text, style, search_state)
    }

    fn modules_span(
        &self,
        search_state: Option<&SearchState>,
        max_len: usize,
    ) -> Option<SpannedString<Style>> {
        let style = mod_style(&self.default_style());
        let mut text;
        match (!self.subtrees.is_empty(), self.subject_module.is_some()) {
            (true, _) => {
                text = ":".to_string();
                let subtree_modules: Vec<String> = self.subtrees.iter().map(|m| m.id()).collect();
                text.push_str(&subtree_modules.join(" :"));
                if text.width() > max_len {
                    text = format!("({} modules)", subtree_modules.len());
                }
            }
            (false, true) => text = self.subject_module.as_ref().unwrap().clone(),
            (false, false) => return None,
        };

        Some(search_if_needed!(text, style, search_state))
    }

    fn graph_span(&self) -> SpannedString<Style> {
        let style = self.default_style();
        let mut result = SpannedString::new();
        for _ in 0..self.level {
            result.append_styled("│ ", style)
        }

        if self.commit.bellow().is_none() {
            result.append_styled("◉", style)
        } else if self.commit.is_commit_link() {
            result.append_styled("⭞", style)
        } else {
            result.append_styled("●", style)
        }

        if self.commit.is_merge() {
            if self.subtree_operation == SubtreeOperation::Import
                || self.subtree_operation == SubtreeOperation::Update
            {
                if self.commit.is_fork_point() {
                    result.append_styled("⇤┤", style);
                } else {
                    result.append_styled("⇤╮", style);
                }
            } else if self.commit.is_fork_point() {
                result.append_styled("─┤", style);
            } else {
                result.append_styled("─┐", style)
            }
        } else if self.commit.is_fork_point() {
            result.append_styled("─┘", style)
        }

        result
    }

    fn subject_span(&self, search_state: Option<&SearchState>) -> SpannedString<Style> {
        let style = self.default_style();
        let text = &self.subject;
        search_if_needed!(text, style, search_state)
    }

    fn references_span(&self, search_state: Option<&SearchState>) -> SpannedString<Style> {
        let style = ref_style(&self.default_style());
        let mut result = SpannedString::new();
        for r in self.commit.references() {
            result.append_styled('«', style);
            if let Some(needle) = search_state {
                let tmp: SpannedString<Style> =
                    HistoryEntry::highlight_search(style, &r.to_string(), needle);
                result.append::<SpannedString<Style>>(tmp);
            } else {
                result.append_styled(&r.to_string(), style);
            }
            result.append_styled("» ", style);
        }
        result
    }

    fn highlight_search(
        style: Style,
        text: &str,
        search_state: &SearchState,
    ) -> SpannedString<Style> {
        let mut cur = 0;
        let mut tmp = SpannedString::new();
        let indices = HistoryEntry::search_text(text, search_state.needle.as_str());
        for s in indices {
            assert!(s.start >= cur);
            if cur < s.start {
                tmp.append_styled(&text[cur..s.start], style)
            }
            cur = s.end;

            tmp.append_styled(&text[s.start..s.end], search_state.style());
        }
        if cur < text.len() - 1 {
            tmp.append_styled(&text[cur..], style)
        }
        tmp
    }

    pub fn render(
        &self,
        search_state: Option<&SearchState>,
        widths: WidthConfig,
    ) -> SpannedString<Style> {
        let style = self.default_style();
        let mut buf = SpannedString::new();

        {
            buf.append(self.id_span(search_state));
            buf.append_styled(" ", style);
        }

        {
            // Author date
            buf.append(self.date_span(widths.max_date));
            buf.append_styled(" ", style);
        }

        {
            // Author name
            buf.append(self.name_span(search_state, widths.max_author));
            buf.append_styled(" ", style);
        }

        buf.append_styled(self.commit.icon(), style);

        buf.append(self.graph_span());
        buf.append_styled(" ", style);

        if let Some(modules) = self.modules_span(search_state, widths.max_modules) {
            buf.append(modules);
            buf.append_styled(" ", style);
        }
        {
            buf.append(self.subject_span(search_state));
            buf.append_styled(" ", style);
        }
        buf.append(self.references_span(search_state));

        buf
    }
}

pub fn split_subject(subject: &String) -> (Option<String>, Option<String>) {
    let reg = regex!(r"^\w+\((.+)\): .+");
    let mut subject_module = None;
    let mut short_subject = None;
    if let Some(caps) = reg.captures(&subject) {
        let x = caps.get(1).expect("Expected 1 capture group");
        subject_module = Some(x.as_str().to_string());
        let mut f = subject.clone();
        f.truncate(x.start() - 1);
        f.push_str(&subject.clone().split_off(x.end() + 1));
        short_subject = Some(f);
    }
    (subject_module, short_subject)
}

pub trait DisplayableCommit {
    fn commit(&self) -> &Commit;
}

// I'm not proud of this code. Ohh Omnissiah be merciful on my soul‼
fn adjust_string(text: &str, len: usize) -> String {
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
