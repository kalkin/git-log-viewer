use cursive::theme::*;
use cursive::utils::span::{SpannedStr, SpannedString};
use glv_core::*;
use posix_errors::PosixError;

use cursive::direction::Direction;

pub struct History {
    history: Vec<Commit>,
}

impl History {
    pub fn new(working_dir: &str, range: &str) -> Result<History, PosixError> {
        let history = commits_for_range(working_dir, range, 0, None, vec![], Some(0), Some(100))?;
        assert!(history.len() > 0);
        Ok(History { history })
    }
}

impl cursive::view::View for History {
    fn draw(&self, printer: &cursive::Printer) {
        let height = printer.size.y;
        let mut iter = self.history.iter();
        let mut default_style: Style = Default::default();
        default_style.color = ColorStyle::terminal_default();

        for i in printer.offset.x..printer.offset.x + height {
            let mut buf = SpannedString::new();
            if let Some(commit) = iter.next() {
                {
                    let mut id_style = Style::none();
                    id_style.color = ColorStyle::new(
                        Color::Dark(BaseColor::Magenta),
                        Color::TerminalDefault
                        );
                    buf.append_styled(commit.short_id(), id_style);
                }
                buf.append_styled(" ", default_style);
                {
                    let mut date_style = Style::none();
                    date_style.color = ColorStyle::new(
                        Color::Dark(BaseColor::Blue),
                        Color::TerminalDefault
                        );
                    buf.append_styled(commit.author_date(), date_style);
                }

                buf.append_styled(" ", default_style);
                {
                    let mut name_style = Style::none();
                    name_style.color = ColorStyle::new(
                        Color::Dark(BaseColor::Green),
                        Color::TerminalDefault
                        );
                    buf.append_styled(commit.author_name(), name_style);
                }

                buf.append_styled(" ", default_style);
                {
                    buf.append(commit.subject());
                }

                buf.append_styled(" ", default_style);
                {
                    let mut ref_style = Style::none();
                    ref_style.color = ColorStyle::new(
                        Color::Dark(BaseColor::Yellow),
                        Color::TerminalDefault
                        );
                    for r in commit.references() {
                        buf.append_styled(r.to_string(), ref_style);
                    }
                }
            }
            let rest = printer.size.x - buf.width();
            if rest > 0 {
                for j in 0 .. rest {
                    buf.append(" ");
                }
            }
            let t = SpannedStr::from(&buf);
            printer.print_styled((0, i), t);
        }
    }
    fn take_focus(&mut self, _: Direction) -> bool {
        true
    }
}
