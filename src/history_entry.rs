use cursive::theme::Style;
use cursive::utils::span::SpannedString;
use unicode_width::UnicodeWidthStr;

use glv_core::Commit;

use crate::search::SearchState;
use crate::style::{date_style, id_style, mod_style, name_style, ref_style};

pub struct HistoryEntry<'a, 'b> {
    commit: &'a Commit,
    default_style: Style,
    search_state: &'b SearchState,
    width_config: WidthConfig,
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

impl<'a, 'b> HistoryEntry<'a, 'b> {
    pub fn new(
        default_style: Style,
        commit: &'a Commit,
        search_state: &'b SearchState,
        width_config: WidthConfig,
    ) -> HistoryEntry<'a, 'b> {
        HistoryEntry {
            commit,
            default_style,
            search_state,
            width_config,
        }
    }

    pub fn name(&self) -> SpannedString<Style> {
        let style = name_style(&self.default_style);
        let text = glv_core::adjust_string(self.commit.author_name(), self.width_config.max_author);
        let mut result = SpannedString::new();
        if self.search_state.active {
            result = <HistoryEntry<'a, 'b>>::highlight_search(style, &text, &self.search_state);
        } else {
            result.append_styled(text, style);
        }
        result
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

    pub fn date(&self) -> SpannedString<Style> {
        let style = date_style(&self.default_style);
        let text =
            glv_core::adjust_string(self.commit.author_rel_date(), self.width_config.max_date);
        let mut result = SpannedString::new();
        result.append_styled(text, style);
        result
    }

    pub fn id(&self) -> SpannedString<Style> {
        let style = id_style(&self.default_style);
        let text = self.commit.short_id();
        let mut result;
        if self.search_state.active {
            result = <HistoryEntry<'a, 'b>>::highlight_search(style, &text, &self.search_state);
        } else {
            result = SpannedString::new();
            result.append_styled(text, style);
        }
        result
    }

    pub fn modules(&self) -> Option<SpannedString<Style>> {
        let style = mod_style(&self.default_style);
        let mut text;
        match (
            !self.commit.subtree_modules().is_empty(),
            self.commit.subject_module().is_some(),
        ) {
            (true, _) => {
                text = ":".to_string();
                let subtree_modules = self.commit.subtree_modules();
                text.push_str(&subtree_modules.join(" :"));
                if text.width() > self.width_config.max_modules {
                    text = format!("({} modules)", subtree_modules.len());
                }
            }
            (false, true) => text = self.commit.subject_module().unwrap().clone(),
            (false, false) => return None,
        };

        Some(<HistoryEntry<'a, 'b>>::highlight_search(style, &text, &self.search_state))
    }

    pub fn graph(&self) -> SpannedString<Style> {
        let style = self.default_style;
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
            if self.commit.subject().starts_with("Update :")
                || self.commit.subject().contains(" Import ")
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

    pub fn subject(&self) -> SpannedString<Style> {
        let style = self.default_style;
        let text = if let Some(v) = self.commit.short_subject() {
            v
        } else {
            self.commit.subject()
        };

        let mut result;
        if self.search_state.active {
            let search_state = self.search_state;
            result = <HistoryEntry<'a, 'b>>::highlight_search(style, &text, search_state);
        } else {
            result = SpannedString::new();
            result.append_styled(text, style);
        }

        result
    }

    pub fn references(&self) -> SpannedString<Style> {
        let style = ref_style(&self.default_style);
        let mut result = SpannedString::new();
        for r in self.commit.references() {
            result.append_styled('«', style);
            if self.search_state.active {
                let search_state = self.search_state;
                let tmp: SpannedString<Style> =
                    <HistoryEntry<'a, 'b>>::highlight_search(style, &r.to_string(), search_state);
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
        let indices = <HistoryEntry<'a, 'b>>::search_text(text, search_state.needle.as_str());
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
}
