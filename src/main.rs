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

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::{env, io};

use clap::{Parser, ValueHint};
use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyModifiers};

use git_wrapper::Repository;

use history_adapter::HistoryAdapter;
use history_entry::HistoryEntry;
use memory_logger::blocking::MemoryLogger;
use ui::base::Drawable;

use crate::detail::DiffView;
use crate::history_table::TableWidget;
use crate::ui::base::{
    new_area, render, setup_screen, shutdown_screen, Area, HandleEvent, StyledArea,
};
use crate::ui::layouts::SplitLayout;
use crossterm::ErrorKind;
use posix_errors::PosixError;
use std::process::exit;
use std::time::{Duration, Instant};

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

    let mut debug = true;
    let log_level = match args.debug {
        0 => {
            debug = false;
            log::Level::Warn
        }
        1 => log::Level::Info,
        2 => log::Level::Debug,
        _ => log::Level::Trace,
    };
    let logger = memory_logger::blocking::MemoryLogger::setup(log_level)
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

    let repo =
        Repository::from_args(args.change_dir.as_deref(), None, None).map_err(PosixError::from)?;

    let (revisions, paths): (Vec<OsString>, Vec<PathBuf>) =
        parse_rev_paths(&repo, args.revision, &args.paths)?;
    log::info!("Revs  {:?}", revisions);
    log::info!("Paths {:?}", paths);
    let history_adapter = HistoryAdapter::new(repo.clone(), revisions, paths.clone(), debug)?;

    run_ui(history_adapter, repo, paths, logger).map_err(Into::into)
}

#[allow(unused_qualifications)]
#[allow(clippy::panic_in_result_fn)]
fn parse_rev_paths<S: AsRef<OsStr> + std::fmt::Debug + std::convert::From<String>>(
    repo: &Repository,
    in_rev: Vec<S>,
    in_paths: &[PathBuf],
) -> Result<(Vec<S>, Vec<PathBuf>), PosixError>
where
    PathBuf: From<S>,
{
    assert!(
        !in_rev.is_empty(),
        "Revision vec should contain at least 'HEAD'"
    );
    let mut revisions = Vec::with_capacity(in_rev.len());
    if in_paths.is_empty() {
        // validate if there are revisions or paths
        let mut paths: Vec<PathBuf> = vec![];
        let mut parsing_revisions = true;
        for rev in in_rev {
            if parsing_revisions && is_valid_rev_spec(repo, &rev) {
                revisions.push(rev);
            } else if parsing_revisions {
                parsing_revisions = false;
                paths.push(rev.into());
            } else {
                paths.push(rev.into());
            }
        }
        let normalized_paths = normalize_paths(repo, &paths);
        if revisions.is_empty() {
            revisions.push("HEAD".to_owned().into());
        }
        Ok((revisions, normalized_paths))
    } else {
        for rev in in_rev {
            if is_valid_rev_spec(repo, &rev) {
                revisions.push(rev);
            } else {
                return Err(PosixError::new(
                    1,
                    format!("Invalid revision spec '{:?}'", rev),
                ));
            }
        }
        let paths = normalize_paths(repo, in_paths);
        Ok((revisions, paths))
    }
}

fn is_valid_rev_spec<S: AsRef<OsStr>>(repo: &Repository, rev: &S) -> bool {
    let mut git = repo.git();
    git.args(&["rev-parse", "-q"]).arg(rev).arg("--");
    let proc = git.output().expect("Failed to run rev-parse");

    proc.status.success()
}

fn normalize_paths(repo: &Repository, paths: &[PathBuf]) -> Vec<PathBuf> {
    match (repo.work_tree(), env::current_dir()) {
        (Some(work_tree), Ok(cwd)) => {
            if let Ok(prefix) = cwd.strip_prefix(work_tree) {
                // glv was executed inside the work_tree
                paths
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
                paths.to_vec()
            }
        }
        (_, _) => paths.to_vec(),
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
    logger: &MemoryLogger,
) -> Result<(), ErrorKind> {
    let root = build_drawable(repo, history_adapter, paths);
    let result = ui_loop(root);

    let contents = logger.read().to_string();
    #[allow(clippy::print_stderr)]
    for line in contents.lines() {
        eprintln!("{}", line);
    }
    result
}

fn ui_loop(
    mut drawable: SplitLayout<TableWidget, DiffView, HistoryEntry>,
) -> Result<(), io::Error> {
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
    let mut area = new_area();
    let mut last_rendered = drawable.render(&area);
    setup_screen("glv")?;
    render(&last_rendered, &area)?;
    // We start with 10ms timeout and bump it up everytime we timeout and rendering doesn't show
    // any updates. The idea is that every time we render and see no changes we bump the timer up
    // to 1 second in 100 ms steps.
    let mut timeout = Duration::from_millis(10);
    loop {
        match rx.recv_timeout(timeout) {
            Ok(event) => {
                let start = Instant::now();
                log::debug!(target:"main:ui_loop", "Received Event {:?}", event);
                if drawable.on_event(&event) == HandleEvent::Ignored {
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
                        _ => {
                            log::info!(target:"main:ui_loop", "Unexpected event: {:?}", event);
                        }
                    }
                }
                if area.height() >= 4 && area.width() >= 10 {
                    let new = drawable.render(&area);
                    log::trace!(target:"main:ui_loop", "Set recv timeout to {:?}", timeout);
                    if same(&new, &last_rendered) {
                        log::debug!(target:"main:ui_loop", "Skipping useless rendering calculation");
                    } else {
                        last_rendered = new;
                        render(&last_rendered, &area)?;
                    }
                } else {
                    log::warn!(target:"main:ui_loop", "target area too small");
                }

                let duration = start.elapsed();
                if duration.as_millis() > 50 {
                    log::warn!(target:"main:ui_loop", "Runtime {:?} !", duration);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let start = Instant::now();
                let new = drawable.render(&area);
                #[allow(clippy::else_if_without_else)]
                if area.height() >= 4 && area.width() >= 10 {
                    if !same(&new, &last_rendered) {
                        last_rendered = new;
                        render(&last_rendered, &area)?;
                    } else if Duration::from_millis(1000) > timeout {
                        timeout = timeout.saturating_add(Duration::from_millis(100));
                        log::trace!(target:"main:ui_loop","set recv timeout to {:?}", timeout);
                    }
                } else {
                    log::warn!(target:"main:ui_loop","target area too small");
                }

                let duration = start.elapsed();
                if duration.as_millis() > 50 {
                    log::warn!(target:"main:ui_loop", "Runtime {:?} !", duration);
                }
            }
            Err(err) => {
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionAborted,
                    format!("Event loop disconnected:\n{:?}", err),
                ))
            }
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
    /// Run as if was started in <path>
    #[clap(short = 'C', takes_value = true, value_hint=ValueHint::DirPath)]
    pub change_dir: Option<String>,

    /// Branch, tag or commit id
    #[clap(default_value = "HEAD")]
    revision: Vec<OsString>,

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
        let _args: Args =
            Parser::try_parse_from(&["glv", "master", "foo/bar"]).expect("Should accept it");
    }
}
