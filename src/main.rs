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

use std::sync::mpsc;
use std::thread;

use clap::{app_from_crate, App, Arg};
use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyModifiers};

use git_wrapper::Repository;

use history_adapter::HistoryAdapter;
use history_entry::HistoryEntry;

use crate::detail::DiffView;
use crate::history_table::TableWidget;
use crate::ui::base::{
    new_area, render, setup_screen, shutdown_screen, Area, Drawable, HandleEvent, StyledArea,
};
use crate::ui::layouts::SplitLayout;
use crossterm::ErrorKind;
use posix_errors::PosixError;
use std::process::exit;
use std::sync::mpsc::TryRecvError;
use std::time;

mod actors;
#[macro_use]
mod commit;
pub mod default_styles;
mod detail;
mod history_adapter;
pub mod history_entry;
mod history_table;
mod raw;
mod search;
mod ui;

#[allow(clippy::ptr_arg)]
fn same(a: &StyledArea<String>, b: &StyledArea<String>) -> bool {
    if a.len() != b.len() {
        return false;
    }
    for (i, a_line) in a.iter().enumerate() {
        let b_line = &b[i];
        if a_line.len() != b_line.len() {
            return false;
        }
        for (j, a_sc) in a_line.iter().enumerate() {
            let b_sc = &b_line[j];
            if a_sc.content() != b_sc.content() || a_sc.style() != b_sc.style() {
                return false;
            }
        }
    }
    true
}

struct UiError(PosixError);
impl From<ErrorKind> for UiError {
    fn from(err: ErrorKind) -> Self {
        UiError(PosixError::from(err))
    }
}

fn glv() -> Result<(), PosixError> {
    let app = arg_parser();

    let matches = app.get_matches();
    let repo_tmp = Repository::from_args(
        matches.value_of("dir"),
        matches.value_of("git-dir"),
        matches.value_of("working-tree"),
    );

    if let Err(err) = repo_tmp {
        let msg = format!("{}", err);
        return Err(PosixError::new(128, msg));
    }

    let repo = repo_tmp.unwrap();

    let revision = matches.value_of("REVISION").unwrap();

    let paths = if let Some(p) = matches.values_of("path") {
        p.map(ToString::to_string).collect()
    } else {
        vec![]
    };

    let history_adapter = HistoryAdapter::new(repo.clone(), revision, paths.clone())?;
    if let Err(err) = run_ui(history_adapter, repo, paths) {
        Err(UiError::from(err).0)
    } else {
        Ok(())
    }
}

fn main() {
    if let Err(e) = glv() {
        eprintln!("{}", e.message());
        exit(e.code());
    }
}

fn run_ui(
    history_adapter: HistoryAdapter,
    repo: Repository,
    paths: Vec<String>,
) -> Result<(), crossterm::ErrorKind> {
    let mut area = new_area();
    let (tx, rx) = mpsc::channel::<Event>();
    {
        thread::spawn(move || {
            while let Ok(event) = read() {
                if let Err(err) = tx.send(event) {
                    panic!("Error: {:?}", err);
                }
            }
        });
    }

    let mut drawable = build_drawable(repo, history_adapter, paths);
    let mut last_rendered = drawable.render(&area);

    setup_screen("glv")?;
    render(&last_rendered, &area)?;
    loop {
        match rx.try_recv() {
            Ok(event) => {
                if let HandleEvent::Ignored = drawable.on_event(event) {
                    match event {
                        Event::Resize(cols, rows) => {
                            area = Area::new(cols as usize, rows as usize);
                        }
                        Event::Key(KeyEvent {
                            code: KeyCode::Char('q'),
                            modifiers: KeyModifiers::NONE,
                        }) => {
                            break;
                        }
                        _ => {}
                    }
                }
                let new = drawable.render(&area);
                if !same(&new, &last_rendered) {
                    last_rendered = new;
                    render(&last_rendered, &area)?;
                }
            }
            Err(err) => match err {
                TryRecvError::Empty => {
                    let new = drawable.render(&area);
                    if !same(&new, &last_rendered) {
                        last_rendered = new;
                        render(&last_rendered, &area)?;
                    }
                    let hundred_millis = time::Duration::from_millis(100);
                    thread::sleep(hundred_millis);
                }
                TryRecvError::Disconnected => {
                    panic!("Something gone wrong: {:?}", err);
                }
            },
        }
    }

    shutdown_screen()?;
    Ok(())
}

fn arg_parser() -> App<'static> {
    let dir_arg = Arg::new("dir")
        .short('C')
        .takes_value(true)
        .help("Change to <dir> before start");
    let w_arg = Arg::new("working-tree")
        .long("work-tree")
        .takes_value(true)
        .help("Directory where the GIT_WORK_TREE is.");
    let gd_arg = Arg::new("git-dir")
        .long("git-dir")
        .takes_value(true)
        .help("Directory where the GIT_DIR is.");
    let rev_arg = Arg::new("REVISION")
        .help("Branch, tag or commit id")
        .default_value("HEAD")
        .required(false);
    let paths_arg = Arg::new("path")
        .help("Show only commits touching the paths")
        .multiple_values(true)
        .last(true);
    app_from_crate!()
        .arg(dir_arg)
        .arg(w_arg)
        .arg(gd_arg)
        .arg(rev_arg)
        .arg(paths_arg)
}

fn build_drawable(
    repo: Repository,
    history_adapter: HistoryAdapter,
    paths: Vec<String>,
) -> SplitLayout<TableWidget, DiffView, HistoryEntry> {
    let history_list = { TableWidget::new(history_adapter) };
    let diff = DiffView::new(repo, paths);

    SplitLayout::new(history_list, diff)
}

#[test]
fn verify_app() {
    arg_parser().debug_assert();
}
