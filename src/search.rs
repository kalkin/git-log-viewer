use cursive::theme::{BaseColor, Color, ColorType, Effect, Style};
use cursive::utils::span::SpannedString;
use unicode_width::UnicodeWidthStr;

use glv_core::Commit;

use crate::style::{date_style, id_style, mod_style, name_style, ref_style};

pub struct SearchState {
    pub active: bool,
    pub needle: String,
    style: Style,
}

impl SearchState {
    pub fn new(default_style: Style) -> Self {
        let mut style = default_style;
        style.color.back = ColorType::Color(Color::Dark(BaseColor::Red));
        style.color.front = ColorType::Color(Color::Dark(BaseColor::White));
        style.effects |= Effect::Bold;
        SearchState {
            active: false,
            needle: "".to_string(),
            style,
        }
    }
}

pub struct SearchableCommit<'a, 'b> {
    commit: &'a Commit,
    default_style: Style,
    search_state: &'b SearchState,
}

impl<'a, 'b> SearchableCommit<'a, 'b> {
    pub fn new(
        default_style: Style,
        commit: &'a Commit,
        search_state: &'b SearchState,
    ) -> SearchableCommit<'a, 'b> {
        SearchableCommit {
            commit,
            default_style,
            search_state,
        }
    }

    pub fn author_name(&self, max: usize) -> SpannedString<Style> {
        let style = name_style(&self.default_style);
        let text = glv_core::adjust_string(self.commit.author_name(), max);
        let mut result = SpannedString::new();
        if self.search_state.active {
            result = <SearchableCommit<'a, 'b>>::highlight_search(style, &text, &self.search_state);
        } else {
            result.append_styled(text, style);
        }
        result
    }

    pub fn author_rel_date(&self, max: usize) -> SpannedString<Style> {
        let style = date_style(&self.default_style);
        let text = glv_core::adjust_string(self.commit.author_rel_date(), max);
        let mut result = SpannedString::new();
        result.append_styled(text, style);
        result
    }

    pub fn short_id(&self) -> SpannedString<Style> {
        let style = id_style(&self.default_style);
        let text = self.commit.short_id();
        let mut result;
        if self.search_state.active {
            result = <SearchableCommit<'a, 'b>>::highlight_search(style, &text, &self.search_state);
        } else {
            result = SpannedString::new();
            result.append_styled(text, style);
        }
        result
    }

    pub fn modules(&self, max: usize) -> Option<SpannedString<Style>> {
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
                if text.width() > max {
                    text = format!("({} modules)", subtree_modules.len());
                }
            }
            (false, true) => text = self.commit.subject_module().unwrap().clone(),
            (false, false) => return None,
        };
        let mut result = SpannedString::new();
        result.append_styled(text, style);
        Some(result)
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
            let search_state = &self.search_state;
            result = <SearchableCommit<'a, 'b>>::highlight_search(style, &text, search_state);
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
                let search_state = &self.search_state;
                let tmp: SpannedString<Style> = <SearchableCommit<'a, 'b>>::highlight_search(
                    style,
                    &r.to_string(),
                    search_state,
                );
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
        let indices = text.match_indices(search_state.needle.as_str());
        for (i, s) in indices {
            assert!(i >= cur);
            if cur < i {
                tmp.append_styled(&text[cur..i], style)
            }
            cur = i + s.len();

            tmp.append_styled(s, search_state.style)
        }
        if cur < text.len() - 1 {
            tmp.append_styled(&text[cur..], style)
        }
        tmp
    }
}
