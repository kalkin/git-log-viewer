use cursive::theme::Style;
use cursive::utils::span::SpannedString;

use cursive::views::{ScrollView, TextContent, TextView};
use cursive::{Cursive, CursiveExt};

mod raw;

fn git_log() -> Vec<SpannedString<Style>> {
    let working_dir = &git_wrapper::top_level().unwrap()[..];
    let proc = git_wrapper::git_cmd_out(
        working_dir.to_string(),
        &["log", "-1", "-p", "--color=always"],
        )
        .unwrap();

    let stdout: Vec<u8> = proc.stdout;
    raw::parse_spans(stdout)
}

fn main() {
    let tmp = git_log();
    // Creates the cursive root - required for every application.
    let mut siv = Cursive::new();
    let content = TextContent::new("");
    for line in tmp {
        content.append(line);
    }

    let view = TextView::new_with_content(content);
    siv.add_fullscreen_layer(ScrollView::new(view));

    siv.add_global_callback('q', |s| s.quit());

    siv.run();
}
