use crossterm::style::{Color, ContentStyle};
lazy_static::lazy_static! {
    pub static ref DEFAULT_STYLE: ContentStyle = ContentStyle::new();
    pub static ref ID_STYLE: ContentStyle = (*DEFAULT_STYLE).foreground(Color::DarkMagenta);
    pub static ref DATE_STYLE: ContentStyle = (*DEFAULT_STYLE).foreground(Color::DarkBlue);
    pub static ref MOD_STYLE: ContentStyle = (*DEFAULT_STYLE).foreground(Color::DarkYellow);
    pub static ref NAME_STYLE: ContentStyle = (*DEFAULT_STYLE).foreground(Color::DarkGreen);
    pub static ref REF_STYLE: ContentStyle = (*DEFAULT_STYLE).foreground(Color::DarkCyan);
}
