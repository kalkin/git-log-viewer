use cursive::theme::{PaletteColor::*, Style};
use cursive::utils::span::SpannedString;

use cursive::traits::*;
use cursive::views::*;
use cursive::{Cursive, CursiveExt};

mod history;
mod raw;

fn git_log() -> Vec<SpannedString<Style>> {
    let working_dir = &git_wrapper::top_level().unwrap()[..];
    let proc = git_wrapper::git_cmd_out(
        working_dir.to_string(),
        vec!["log", "-1", "-p", "--color=always"],
    )
    .unwrap();

    let stdout: Vec<u8> = proc.stdout;
    raw::parse_spans(stdout)
}

fn main() {
    cursive::logger::init();
    // Creates the cursive root - required for every application.
    let mut siv = Cursive::new();

    //let tmp = git_log();
    //let content = TextContent::new("");
    //for line in tmp {
    //content.append(line);
    //}

    //let diff_view = TextView::new_with_content(content)
    //.full_width()
    //.scrollable();

    let working_dir = git_wrapper::top_level().unwrap();
    let history_log = history::History::new(&working_dir, "HEAD")
        .unwrap()
        .full_screen()
        .scrollable();

    let ll = LinearLayout::vertical().child(history_log);
    //.child(diff_view);
    siv.add_fullscreen_layer(ll);
    siv.add_global_callback('q', |s| s.quit());
    let mut theme: cursive::theme::Theme = Default::default();
    theme.palette[View] = cursive::theme::Color::TerminalDefault;
    theme.palette[Primary] = cursive::theme::Color::TerminalDefault;
    siv.set_theme(theme);
    siv.add_global_callback('~', cursive::Cursive::toggle_debug_console);
    log::error!("Something serious probably happened!");

    siv.run();
}
