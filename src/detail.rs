use cursive::event::{Event, EventResult};
use cursive::traits::*;
use cursive::views::{ScrollView, TextContent, TextView};
use cursive::{Printer, Vec2, View};

use glv_core::Commit;

use crate::raw;
use crate::views::DetailView;

pub struct CommitDetailView {
    content: Option<ScrollView<TextView>>,
}

impl CommitDetailView {
    pub fn new() -> Self {
        CommitDetailView { content: None }
    }
}

impl View for CommitDetailView {
    fn draw(&self, printer: &Printer) {
        if self.content.is_some() {
            self.content.as_ref().unwrap().draw(printer);
        }
    }

    fn layout(&mut self, size: Vec2) {
        if self.content.is_some() {
            let content = self.content.as_mut().unwrap();
            content.layout(size)
        }
    }

    fn on_event(&mut self, e: Event) -> EventResult {
        assert!(self.content.is_some());
        self.content.as_mut().unwrap().on_event(e)
    }
}

impl DetailView for CommitDetailView {
    fn set_detail(&mut self, detail: &Commit) {
        let tmp = git_log(&detail.id().0);
        self.content = Some(TextView::new_with_content(tmp).scrollable());
    }
}

fn git_log(id: &str) -> TextContent {
    let working_dir = &git_wrapper::top_level().unwrap()[..];
    let proc = git_wrapper::git_cmd_out(
        working_dir.to_string(),
        vec!["log", "-1", "-p", "--color=always", id],
    )
    .unwrap();

    let stdout: Vec<u8> = proc.stdout;

    let content = TextContent::new("");
    for line in raw::parse_spans(stdout) {
        content.append(line);
    }
    content
}
