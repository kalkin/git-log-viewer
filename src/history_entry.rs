use cursive::theme::{Effect, Style};
use cursive::utils::span::SpannedString;
use regex::Regex;
use unicode_width::UnicodeWidthStr;

use crate::core::{adjust_string, Commit};
use crate::search::SearchState;
use crate::style::{date_style, id_style, mod_style, name_style, ref_style, DEFAULT_STYLE};

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
pub enum SubtreeType {
    Update,
    Import,
    Split,
    None,
}

pub struct HistoryEntry<'a> {
    commit: &'a Commit,
    subtree_type: SubtreeType,
    subject_module: Option<String>,
    subject: String,
    selected: bool,
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

impl<'a> HistoryEntry<'a> {
    pub fn new(commit: &'a Commit) -> Self {
        let mut subtree_type = SubtreeType::None;
        if commit.subject().starts_with("Update :") {
            subtree_type = SubtreeType::Update
        } else if commit.subject().starts_with("Import :") {
            subtree_type = SubtreeType::Import
        } else if commit.subject().starts_with("Split '") {
            subtree_type = SubtreeType::Split
        }

        let (subject_module, short_subject) = split_subject(&commit.subject());
        let subject = short_subject.unwrap_or_else(|| commit.subject().clone());

        HistoryEntry {
            commit,
            subject,
            subject_module,
            subtree_type,
            selected: false,
        }
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
        match (
            !self.commit.subtree_modules().is_empty(),
            self.subject_module.is_some(),
        ) {
            (true, _) => {
                text = ":".to_string();
                let subtree_modules = self.commit.subtree_modules();
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
        for _ in 0..self.commit.level() {
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
            if self.subtree_type == SubtreeType::Import || self.subtree_type == SubtreeType::Update
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
