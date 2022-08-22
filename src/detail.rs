// Copyright (C) 2021  Bahtiar `kalkin-` Gadimov <bahtiar@gadimov.de>
//
// This file is part of git-log-viewer
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::path::PathBuf;
use std::process::{Command, Stdio};

use crossterm::event::Event;
use crossterm::style::{style, ContentStyle, StyledContent};

use git_wrapper::Repository;

use crate::commit::Commit;
use crate::commit::Oid;
use crate::default_styles::{
    DATE_STYLE, DEBUG_STYLE, DEFAULT_STYLE, ID_STYLE, MOD_STYLE, NAME_STYLE, REF_STYLE,
};
use crate::history_entry::HistoryEntry;
use crate::raw;
use crate::ui::base::data::StyledAreaAdapter;
use crate::ui::base::{Area, Drawable, HandleEvent, ListWidget, StyledArea, StyledLine};
use crate::ui::layouts::DetailsWidget;

pub struct DiffView(ListWidget<String>, Vec<PathBuf>, Repository);

impl DiffView {
    pub fn new(repo: Repository, paths: Vec<PathBuf>) -> Self {
        let adapter = StyledAreaAdapter {
            content: vec![],
            thread: None,
        };
        Self(ListWidget::new(Box::new(adapter)), paths, repo)
    }
}

impl Drawable for DiffView {
    fn render(&mut self, area: &Area) -> StyledArea<String> {
        self.0.render(area)
    }

    fn on_event(&mut self, event: &Event) -> HandleEvent {
        self.0.on_event(event)
    }
}

impl DetailsWidget<HistoryEntry> for DiffView {
    fn set_content(&mut self, content: &HistoryEntry) {
        let mut data: StyledArea<String> = vec![
            color_text("Commit:          ", &content.id().0, *ID_STYLE),
            color_text(
                "Parents:         ",
                &content
                    .commit()
                    .parents()
                    .iter()
                    .map(|p| format!("{:?}", p))
                    .collect::<Vec<String>>()
                    .join(" "),
                *ID_STYLE,
            ),
            color_text("Author:          ", content.author_name(), *NAME_STYLE),
            color_text("Author Date:     ", content.author_date(), *DATE_STYLE),
        ];
        // Committer lines {
        if content.author_name() != content.committer_name() {
            data.push(color_text(
                "Committer:       ",
                content.committer_name(),
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
                content.subtrees().iter().map(|e| e.id().clone()).collect();
            data.push(color_text(
                "Strees:          ",
                &module_names.join(", "),
                *MOD_STYLE,
            ));
        }

        if !content.commit().references().is_empty() {
            let references: Vec<&str> = content
                .filtered_references()
                .iter()
                .map(|r| r.0.as_str())
                .collect();
            data.push(color_text(
                "Refs:            ",
                &references.join(", "),
                *REF_STYLE,
            ));
        }
        if *content.debug() {
            add_debug_content(&mut data, content);
        }

        data.push(StyledLine::empty());
        for subject_line in content.original_subject().trim().lines() {
            data.push(color_text(" ", subject_line, *DEFAULT_STYLE));
        }
        data.push(StyledLine::empty());
        for body_line in content.body().trim().lines() {
            data.push(color_text(" ", body_line, *DEFAULT_STYLE));
        }
        data.push(StyledLine::empty());
        data.push(StyledLine {
            content: vec![style(
                "                                 ❦ ❦ ❦ ❦ ".to_owned(),
            )],
        });
        data.push(StyledLine::empty());
        for line in git_diff(&self.2, content.commit(), self.1.as_ref()) {
            data.push(line);
        }
        let adapter = StyledAreaAdapter {
            content: data,
            thread: None,
        };
        self.0 = ListWidget::new(Box::new(adapter));
    }
}

fn add_debug_content(data: &mut Vec<StyledLine<String>>, content: &HistoryEntry) {
    data.push(StyledLine {
        content: vec![style("                                 DEBUG".to_owned())],
    });
    data.push(color_text(
        "top_commit:      ",
        &content.top_commit().to_string(),
        *DEBUG_STYLE,
    ));
    data.push(color_text(
        "fork_point:      ",
        &format!("{:?}", content.fork_point()),
        *DEBUG_STYLE,
    ));
    data.push(color_text(
        "level:           ",
        &content.level().to_string(),
        *DEBUG_STYLE,
    ));
    data.push(color_text(
        "commit_link:     ",
        &content.is_link().to_string(),
        *DEBUG_STYLE,
    ));
    data.push(color_text(
        "is_foldable:     ",
        &content.is_foldable().to_string(),
        *DEBUG_STYLE,
    ));
    if content.is_foldable() {
        data.push(color_text(
            "is_folded:       ",
            &content.is_folded().to_string(),
            *DEBUG_STYLE,
        ));
        if !content.is_folded() {
            data.push(color_text(
                "children:        ",
                &content.visible_children().to_string(),
                *DEBUG_STYLE,
            ));
        }
    }
    data.push(StyledLine {
        content: vec![style(
            "                                 ❦ ❦ ❦ ❦ ".to_owned(),
        )],
    });
}

fn git_diff(repo: &Repository, commit: &Commit, paths: &[PathBuf]) -> Vec<StyledLine<String>> {
    let empty_tree = Oid("4b825dc642cb6eb9a060e54bf8d69288fbee4904".to_owned());
    let bellow = commit.bellow().as_ref().unwrap_or(&empty_tree);
    let rev = format!("{}..{}", bellow.0, commit.id().0);
    let mut cmd = repo.git();
    cmd.args(&[
        "diff",
        "--color=always",
        "--stat",
        "-p",
        "-M",
        "--full-index",
        &rev,
    ]);
    if !paths.is_empty() {
        cmd.arg("--");
        cmd.args(paths);
    }

    if which::which("delta").is_ok() {
        let proc = cmd.stdout(Stdio::piped()).spawn().unwrap();

        let delta_p = Command::new("delta")
            .arg("--paging=never")
            .stdin(Stdio::from(proc.stdout.unwrap()))
            .output()
            .unwrap();
        raw::parse_spans(delta_p.stdout)
    } else {
        let proc = cmd
            .args(paths)
            .output()
            .expect("Failed to execute git-diff(1)");
        raw::parse_spans(proc.stdout)
    }
}

fn color_text(key: &str, value: &str, style: ContentStyle) -> StyledLine<String> {
    let content = format!("{}{}", key, value);
    StyledLine {
        content: vec![StyledContent::new(style, content)],
    }
}
