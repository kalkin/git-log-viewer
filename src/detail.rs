use std::process::{Command, Stdio};

use cursive::event::{Event, EventResult};
use cursive::theme::Style;
use cursive::traits::*;
use cursive::utils::span::SpannedString;
use cursive::views::{ScrollView, TextContent, TextView};
use cursive::{Printer, Vec2, View};

use crate::core::Commit;
use crate::core::Oid;
use crate::history_entry::HistoryEntry;
use crate::raw;
use crate::style::{bold_style, date_style, DEFAULT_STYLE};
use crate::style::{id_style, name_style};
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
        match e {
            Event::Char('/') | Event::Char('?') => {
                log::warn!("Search in diff view NIY");
                EventResult::Consumed(None)
            }
            _ => self.content.as_mut().unwrap().on_event(e),
        }
    }
}

impl DetailView for CommitDetailView {
    fn set_detail(&mut self, entry: &HistoryEntry) {
        let detail = entry.commit();
        let content = TextContent::new("");
        content.append(color_span(
            "Commit:          ",
            &detail.id().0,
            id_style(&DEFAULT_STYLE),
        ));

        content.append(color_span(
            "Author:          ",
            &detail.author_name(),
            name_style(&DEFAULT_STYLE),
        ));

        content.append(color_span(
            "Author Date:     ",
            &detail.author_date(),
            date_style(&DEFAULT_STYLE),
        ));

        // Committer lines {
        if detail.author_name() != detail.committer_name() {
            content.append(color_span(
                "Committer:       ",
                &detail.author_name(),
                name_style(&DEFAULT_STYLE),
            ));
        }
        if detail.author_date() == detail.committer_date() {
            content.append(color_span(
                "Committer Date:  ",
                &detail.committer_date(),
                date_style(&DEFAULT_STYLE),
            ));
        }
        // Committer lines }

        // Modules
        if !entry.subtree_modules().is_empty() {
            content.append(color_span(
                "Modules:         ",
                &entry.subtree_modules().join(", "),
                date_style(&DEFAULT_STYLE),
            ));
        }

        // Subject
        content.append("\n");
        content.append(SpannedString::styled(
            format!(" {}\n", detail.subject()),
            bold_style(&DEFAULT_STYLE),
        ));
        content.append("\n");
        for line in detail.body().lines() {
            content.append(format!(" {}\n", line));
        }
        content.append("                                 ❦ ❦ ❦ ❦ \n\n");
        for s in git_diff(detail) {
            content.append(s);
        }
        self.content = Some(TextView::new_with_content(content).scrollable());
    }
}

fn color_span(key: &str, content: &str, style: Style) -> SpannedString<Style> {
    let line = format!("{}{}\n", key, content);
    SpannedString::styled(line, style)
}

fn git_diff(commit: &Commit) -> Vec<SpannedString<Style>> {
    let working_dir = &git_wrapper::top_level().unwrap()[..];
    let default = Oid { 0: "".to_string() };
    let bellow = commit.bellow().unwrap_or(&default);
    let rev = format!("{}..{}", bellow.0, commit.id().0);
    if which::which("delta").is_ok() {
        let proc = Command::new("git")
            .args(&["-C", working_dir])
            .args(&[
                "diff",
                "--color=always",
                "--stat",
                "-p",
                "-M",
                "--full-index",
                &rev,
            ])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let delta_p = Command::new("delta")
            .arg("--paging=never")
            .stdin(Stdio::from(proc.stdout.unwrap()))
            .output()
            .unwrap();
        raw::parse_spans(delta_p.stdout)
    } else {
        let proc = git_wrapper::git_cmd_out(
            working_dir.to_string(),
            vec![
                "diff",
                "--color=always",
                "--stat",
                "-p",
                "-M",
                "--full-index",
                &rev,
            ],
        )
        .unwrap();
        raw::parse_spans(proc.stdout)
    }
}
