#![allow(clippy::module_name_repetitions)]

use cursive::theme::{BaseColor, Color, ColorStyle, Effect, Style};
use cursive::utils::span::SpannedString;
use lazy_static::lazy_static;

#[must_use]
pub fn id_style(default: &Style) -> Style {
    let mut id_style = *default;
    id_style.color = ColorStyle::new(Color::Dark(BaseColor::Magenta), Color::TerminalDefault);
    id_style
}

#[must_use]
pub fn date_style(default: &Style) -> Style {
    let mut date_style = *default;
    date_style.color = ColorStyle::new(Color::Dark(BaseColor::Blue), Color::TerminalDefault);
    date_style
}

#[must_use]
pub fn name_style(default: &Style) -> Style {
    let mut name_style = *default;
    name_style.color = ColorStyle::new(Color::Dark(BaseColor::Green), Color::TerminalDefault);
    name_style
}

#[must_use]
pub fn ref_style(default: &Style) -> Style {
    let mut ref_style = *default;
    ref_style.color = ColorStyle::new(Color::Dark(BaseColor::Yellow), Color::TerminalDefault);
    ref_style
}

#[must_use]
pub fn mod_style(default: &Style) -> Style {
    let mut ref_style = *default;
    ref_style.color = ColorStyle::new(Color::Dark(BaseColor::Yellow), Color::TerminalDefault);
    ref_style
}

#[must_use]
pub fn bold_style(default: &Style) -> Style {
    let mut style = *default;
    style.effects |= Effect::Bold;
    style
}

#[must_use]
pub fn reverse_style(default: &Style) -> Style {
    let mut style = *default;
    style.effects |= Effect::Reverse;
    style
}

lazy_static! {
    pub static ref DEFAULT_STYLE: Style = Style {
        color: ColorStyle::terminal_default(),
        ..Style::default()
    };
}

#[must_use]
pub fn color_span(key: &str, content: &str, style: Style) -> SpannedString<Style> {
    let line = format!("{}{}\n", key, content);
    SpannedString::styled(line, style)
}
