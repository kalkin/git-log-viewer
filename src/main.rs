use cursive::theme::PaletteColor::*;
use cursive::{Cursive, CursiveExt};

use crate::detail::CommitDetailView;
use crate::scroll::CustomScrollView;
use crate::views::DynamicSplitView;

use docopt::Docopt;
use serde::Deserialize;

mod detail;
mod history;
mod history_entry;
mod raw;
mod scroll;
mod search;
mod style;
mod views;

const USAGE: &str = "
glv - Git Log Viewer a TUI application with support for folding merges

Usage:
    glv [-w DIR|--workdir=DIR] [<revision>] [ -h | --help ]

Options:
    -w DIR, --workdir=DIR   Directory where the git repository is.
    -h, --help              Show this usage.

Arguments:
    <revision>                A branch, tag or commit
";

#[derive(Debug, Deserialize)]
struct Args {
    flag_workdir: Option<String>,
    arg_revision: Option<String>,
}

fn main() {
    cursive::logger::init();
    // Creates the cursive root - required for every application.
    let mut siv = Cursive::new();

    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    let working_dir = args
        .flag_workdir
        .unwrap_or_else(|| git_wrapper::top_level().unwrap());
    let revision = args.arg_revision.unwrap_or("HEAD".to_string());
    let history = history::History::new(&working_dir, &revision).unwrap();
    // let main = CustomScrollView::new(history);
    let main = CustomScrollView::new(history);
    let aside = CommitDetailView::new();
    let spl_view = DynamicSplitView::new(main, aside);

    siv.add_fullscreen_layer(spl_view);
    siv.add_global_callback('q', |s| s.quit());

    let mut theme: cursive::theme::Theme = Default::default();
    theme.palette[View] = cursive::theme::Color::TerminalDefault;
    theme.palette[Primary] = cursive::theme::Color::TerminalDefault;
    siv.set_theme(theme);

    siv.add_global_callback('~', cursive::Cursive::toggle_debug_console);

    siv.run();
}
