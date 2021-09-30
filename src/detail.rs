use std::process::{Command, Stdio};

use crossterm::event::Event;
use crossterm::style::{style, ContentStyle, StyledContent};

use gsi::SubtreeConfig;

use crate::commit::Commit;
use crate::commit::Oid;
use crate::default_styles::{DATE_STYLE, DEFAULT_STYLE, ID_STYLE, NAME_STYLE};
use crate::history_entry::HistoryEntry;
use crate::raw;
use crate::ui::base::data::StyledAreaAdapter;
use crate::ui::base::{Area, Drawable, HandleEvent, ListWidget, StyledArea, StyledLine};
use crate::ui::layouts::DetailsWidget;

pub struct DiffView(ListWidget<String>, Vec<String>);

impl DiffView {
    pub fn new(paths: Vec<String>) -> Self {
        let adapter = StyledAreaAdapter {
            content: vec![],
            thread: None,
        };
        Self(ListWidget::new(Box::new(adapter)), paths)
    }
}

impl Drawable for DiffView {
    fn render(&mut self, area: &Area) -> StyledArea<String> {
        self.0.render(area)
    }

    fn on_event(&mut self, event: Event) -> HandleEvent {
        self.0.on_event(event)
    }
}

impl DetailsWidget<HistoryEntry> for DiffView {
    fn set_content(&mut self, content: &HistoryEntry) {
        let mut data: StyledArea<String> = vec![
            color_text("Commit:          ", &content.id().0, *ID_STYLE),
            color_text("Author:          ", content.author_name(), *NAME_STYLE),
            color_text("Author Date:     ", content.author_date(), *DATE_STYLE),
        ];
        // Committer lines {
        if content.author_name() != content.committer_name() {
            data.push(color_text(
                "Committer:       ",
                content.author_name(),
                *NAME_STYLE,
            ));
        }

        if content.author_date() != content.committer_date() {
            data.push(color_text(
                "Committer Date:  ",
                content.committer_date(),
                *DATE_STYLE,
            ));
        }
        // Committer lines }

        // Modules
        if !content.subtrees().is_empty() {
            let module_names: Vec<String> =
                content.subtrees().iter().map(SubtreeConfig::id).collect();
            data.push(color_text(
                "Strees:          ",
                &module_names.join(", "),
                *DATE_STYLE,
            ));
        }

        data.push(vec![]);
        for subject_line in content.original_subject().trim().lines() {
            data.push(color_text(" ", subject_line, *DEFAULT_STYLE));
        }
        data.push(vec![]);
        for body_line in content.body().trim().lines() {
            data.push(color_text(" ", body_line, *DEFAULT_STYLE));
        }
        data.push(vec![]);
        data.push(vec![style(
            "                                 ❦ ❦ ❦ ❦ ".to_string(),
        )]);
        data.push(vec![]);
        for line in git_diff(content.working_dir(), content.commit(), self.1.clone()) {
            data.push(line);
        }
        let adapter = StyledAreaAdapter {
            content: data,
            thread: None,
        };
        self.0 = ListWidget::new(Box::new(adapter));
    }
}

fn git_diff(working_dir: &str, commit: &Commit, paths: Vec<String>) -> Vec<StyledLine<String>> {
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
            .arg("--")
            .args(paths)
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
            working_dir,
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

fn color_text(key: &str, value: &str, style: ContentStyle) -> StyledLine<String> {
    let content = format!("{}{}", key, value);
    vec![StyledContent::new(style, content)]
}
