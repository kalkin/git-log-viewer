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

//! An alternative to `tig(1)`/`lazygit(1)` which supports folding merges and is
//! expandable via plugins. The application can resolve the default merge titles
//! done by using GitHub or Bitbucket to the actual pull request names.

use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::{env, io};

use clap::{Parser, ValueHint};
use clap_git_options::GitOptions;
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
mod cache;
mod credentials;
mod default_styles;
mod detail;
mod history_adapter;
mod history_entry;
mod history_table;
mod raw;
mod search;
mod ui;
mod utils;

#[allow(clippy::ptr_arg)]
fn same(a: &StyledArea<String>, b: &StyledArea<String>) -> bool {
    if a.len() != b.len() {
        return false;
    }
    for (i, a_line) in a.iter().enumerate() {
        let b_line = &b[i];
        if a_line != b_line {
            return false;
        }
    }
    true
}

fn glv() -> Result<(), PosixError> {
    let args = Args::parse();

    let log_level = match args.debug {
        0 => log::Level::Warn,
        1 => log::Level::Info,
        2 => log::Level::Debug,
        _ => log::Level::Trace,
    };
    simple_logger::init_with_level(log_level)
        .map_err(|e| PosixError::new(128, format!("{}", e)))?;

    log::info!("Log Level is set to {}", log::max_level());

    #[cfg(feature = "update-informer")]
    {
        use update_informer::{registry, Check};
        let informer =
            update_informer::new(registry::GitHub, "kalkin/glv", env!("CARGO_PKG_VERSION"));
        if let Ok(Some(version)) = informer.check_version() {
            log::error!("New version is available: {}", version);
        }
    }

    let repo = Repository::try_from(&args.git).map_err(PosixError::from)?;

    let paths = normalize_paths(&repo, &args);
    log::debug!(
        "Initialising HistoryAdapter with revision {} & paths {:?})",
        args.revision,
        paths
    );
    let history_adapter = HistoryAdapter::new(repo.clone(), &args.revision, paths.clone())?;
    run_ui(history_adapter, repo, paths).map_err(Into::into)
}

fn normalize_paths(repo: &Repository, args: &Args) -> Vec<PathBuf> {
    match (repo.work_tree(), env::current_dir()) {
        (Some(work_tree), Ok(cwd)) => {
            if let Ok(prefix) = cwd.strip_prefix(work_tree) {
                // glv was executed inside the work_tree
                args.paths
                    .iter()
                    .map(|p| {
                        if let Ok(f) = p.strip_prefix("/") {
                            f.to_path_buf()
                        } else {
                            let mut f = prefix.to_path_buf();
                            f.push(p);
                            f
                        }
                    })
                    .collect()
            } else {
                // glv is executed outside the work tree
                args.paths.clone()
            }
        }
        (_, _) => args.paths.clone(),
    }
}

#[allow(clippy::exit)]
fn main() {
    std::panic::set_hook(Box::new(|p| {
        shutdown_screen().expect("Shutdown screen");
        log::error!("Panic {}", p);
        exit(1);
    }));

    if let Err(e) = glv() {
        log::error!("{}", e);
        exit(e.code());
    }
}

fn run_ui(
    history_adapter: HistoryAdapter,
    repo: Repository,
    paths: Vec<PathBuf>,
) -> Result<(), ErrorKind> {
    let mut area = new_area();
    let (tx, rx) = mpsc::channel::<Event>();
    {
        thread::spawn(move || {
            while let Ok(event) = read() {
                if let Err(err) = tx.send(event) {
                    log::error!("Error setting up UI event stream:\n{:?}", err);
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
                if drawable.on_event(event) == HandleEvent::Ignored {
                    match event {
                        Event::Resize(cols, rows) => {
                            area = Area::new(
                                cols.try_into().expect("u16 to usize"),
                                rows.try_into().expect("u16 to usize"),
                            );
                        }
                        Event::Key(KeyEvent {
                            code: KeyCode::Char('q'),
                            modifiers: KeyModifiers::NONE,
                            ..
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
                    return Err(io::Error::new(
                        io::ErrorKind::ConnectionAborted,
                        format!("Event loop disconnected:\n{:?}", err),
                    ))
                }
            },
        }
    }

    shutdown_screen()?;
    Ok(())
}

#[derive(Parser)]
#[clap(
    author,
    version,
    about = "Git log viewer supporting un/folding merges",
    help_expected = true,
    dont_collapse_args_in_usage = true
)]
struct Args {
    #[clap(flatten)]
    git: GitOptions,

    /// Branch, tag or commit id
    #[clap(default_value = "HEAD")]
    revision: String,

    /// Show only commits touching the paths
    #[clap(last = true, value_hint=ValueHint::AnyPath)]
    paths: Vec<PathBuf>,

    /// Log level up to -ddd
    #[clap(short, long, parse(from_occurrences))]
    debug: i8,
}

fn build_drawable(
    repo: Repository,
    history_adapter: HistoryAdapter,
    paths: Vec<PathBuf>,
) -> SplitLayout<TableWidget, DiffView, HistoryEntry> {
    let history_list = { TableWidget::new(history_adapter) };
    let diff = DiffView::new(repo, paths);

    SplitLayout::new(history_list, diff)
}

#[cfg(test)]
mod parse_args {
    use crate::Args;
    use clap::Parser;

    #[test]
    fn no_arguments() {
        let _args: Args = Parser::try_parse_from(&["glv"]).expect("No arguments");
    }

    #[test]
    fn with_ref() {
        let _args: Args = Parser::try_parse_from(&["glv", "master"]).expect("Ref specified");
    }

    #[test]
    fn with_ref_and_path() {
        let _args1: Args = Parser::try_parse_from(&["glv", "master", "--", "foo/bar"])
            .expect("Ref and path specified");
        let _args2: Args = Parser::try_parse_from(&["glv", "master", "--", "foo/bar", "README.md"])
            .expect("Ref and multiple paths specified");
    }

    #[test]
    fn no_delim_between_ref_and_path() {
        let args: Result<Args, _> = Parser::try_parse_from(&["glv", "master", "foo/bar"]);
        assert!(args.is_err(), "Should fail without delimiter '--'");
    }
}
