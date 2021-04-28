use cursive::theme::{BaseColor, Color, ColorStyle, Effect, Style};
use lazy_static::lazy_static;

pub fn id_style(default: &Style) -> Style {
    let mut id_style = *default;
    id_style.color = ColorStyle::new(Color::Dark(BaseColor::Magenta), Color::TerminalDefault);
    id_style
}

pub fn date_style(default: &Style) -> Style {
    let mut date_style = *default;
    date_style.color = ColorStyle::new(Color::Dark(BaseColor::Blue), Color::TerminalDefault);
    date_style
}

pub fn name_style(default: &Style) -> Style {
    let mut name_style = *default;
    name_style.color = ColorStyle::new(Color::Dark(BaseColor::Green), Color::TerminalDefault);
    name_style
}

pub fn ref_style(default: &Style) -> Style {
    let mut ref_style = *default;
    ref_style.color = ColorStyle::new(Color::Dark(BaseColor::Yellow), Color::TerminalDefault);
    ref_style
}

pub fn mod_style(default: &Style) -> Style {
    let mut ref_style = *default;
    ref_style.color = ColorStyle::new(Color::Dark(BaseColor::Yellow), Color::TerminalDefault);
    ref_style
}

pub fn bold_style(default: &Style) -> Style {
    let mut style = *default;
    style.effects |= Effect::Bold;
    style
}

lazy_static! {
    pub static ref DEFAULT_STYLE: Style = Style {
        color: ColorStyle::terminal_default(),
        ..Style::default()
    };
}
