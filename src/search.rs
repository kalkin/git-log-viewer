use cursive::theme::{BaseColor, Color, ColorType, Effect, Style};

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

    pub fn style(&self) -> Style {
        self.style
    }
}
