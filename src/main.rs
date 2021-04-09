use cursive::theme::PaletteColor::*;
use cursive::{Cursive, CursiveExt};

use crate::detail::CommitDetailView;
use crate::scroll::CustomScrollView;
use crate::views::DynamicSplitView;

mod detail;
mod history;
mod raw;
mod scroll;
mod views;

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
    let history = history::History::new(&working_dir, "HEAD").unwrap();
    // let main = CustomScrollView::new(history);
    let main = CustomScrollView::new(history);
    let aside = CommitDetailView::new();
    let spl_view = DynamicSplitView::new(main, aside);

    // let ll = LinearLayout::vertical().child(history_log);
    //.child(diff_view);
    // siv.add_fullscreen_layer(spl_view.full_screen());
    siv.add_fullscreen_layer(spl_view);
    siv.add_global_callback('q', |s| s.quit());

    let mut theme: cursive::theme::Theme = Default::default();
    theme.palette[View] = cursive::theme::Color::TerminalDefault;
    theme.palette[Primary] = cursive::theme::Color::TerminalDefault;
    siv.set_theme(theme);

    siv.add_global_callback('~', cursive::Cursive::toggle_debug_console);

    siv.run();
}
