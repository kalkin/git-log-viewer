use crossterm::style::{Color, ContentStyle};
lazy_static::lazy_static! {
    pub static ref DEFAULT_STYLE: ContentStyle = ContentStyle::new();
    pub static ref ID_STYLE: ContentStyle = ContentStyle {
        foreground_color: Some(Color::DarkMagenta),
        background_color: DEFAULT_STYLE.background_color,
        attributes: DEFAULT_STYLE.attributes
    };
    pub static ref DATE_STYLE: ContentStyle = ContentStyle {
        foreground_color: Some(Color::DarkBlue),
        background_color: DEFAULT_STYLE.background_color,
        attributes: DEFAULT_STYLE.attributes
    };
    pub static ref MOD_STYLE: ContentStyle = ContentStyle {
        foreground_color: Some(Color::DarkYellow),
        background_color: DEFAULT_STYLE.background_color,
        attributes: DEFAULT_STYLE.attributes
    };
    pub static ref NAME_STYLE: ContentStyle = ContentStyle {
        foreground_color: Some(Color::DarkGreen),
        background_color: DEFAULT_STYLE.background_color,
        attributes: DEFAULT_STYLE.attributes
    };
    pub static ref REF_STYLE: ContentStyle = ContentStyle {
        foreground_color: Some(Color::DarkCyan),
        background_color: DEFAULT_STYLE.background_color,
        attributes: DEFAULT_STYLE.attributes
    };
}
