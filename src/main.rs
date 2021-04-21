use clap::{app_from_crate, Arg};
use cursive::theme::PaletteColor::{Primary, View};
use cursive::{Cursive, CursiveExt};

use crate::detail::CommitDetailView;
use crate::scroll::CustomScrollView;
use crate::views::DynamicSplitView;

#[macro_use]
mod commit;
mod config;
mod detail;
mod fork_point;
mod github;
mod history;
mod history_entry;
mod raw;
mod scroll;
mod search;
mod style;
mod subtrees;
mod views;

fn main() {
    let working_dir: String;
    let mut paths = Vec::new();

    let w_arg = Arg::new("working_dir")
        .long("working-dir")
        .short('w')
        .takes_value(true)
        .about("Directory where the git repository is.");
    let rev_arg = Arg::new("REVISION")
        .about("Branch, tag or commit id")
        .default_value("HEAD")
        .required(false);
    let paths_arg = Arg::new("path")
        .about("Show only commits touching the paths")
        .multiple(true)
        .last(true);
    let app = app_from_crate!().arg(w_arg).arg(rev_arg).arg(paths_arg);

    let matches = app.get_matches();

    if let Some(wd) = matches.value_of("working_dir") {
        working_dir = wd.to_string();
    } else {
        working_dir = git_wrapper::top_level().unwrap();
    }
    let revision = matches.value_of("REVISION").unwrap();

    if let Some(p) = matches.values_of("path") {
        paths = p.map(ToString::to_string).collect();
    }

    cursive::logger::init();
    // Creates the cursive root - required for every application.
    let mut siv = Cursive::new();

    let history = history::History::new(&working_dir, &revision, paths).unwrap();
    let main = CustomScrollView::new(history);
    let aside = CommitDetailView::new();
    let spl_view = DynamicSplitView::new(main, aside);

    siv.add_fullscreen_layer(spl_view);
    siv.add_global_callback('q', Cursive::quit);

    let mut theme: cursive::theme::Theme = cursive::theme::Theme::default();
    theme.palette[View] = cursive::theme::Color::TerminalDefault;
    theme.palette[Primary] = cursive::theme::Color::TerminalDefault;
    siv.set_theme(theme);

    siv.add_global_callback('~', cursive::Cursive::toggle_debug_console);

    siv.set_fps(20);
    siv.run();
}
