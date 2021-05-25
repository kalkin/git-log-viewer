use std::sync::mpsc;
use std::thread;

use clap::{app_from_crate, App, Arg};
use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyModifiers};

use history_adapter::HistoryAdapter;
use history_entry::HistoryEntry;

use crate::detail::DiffView;
use crate::history_table::TableWidget;
use crate::ui::base::{
    new_area, render, setup_screen, shutdown_screen, Area, Drawable, HandleEvent, StyledArea,
};
use crate::ui::layouts::SplitLayout;
use crossterm::ErrorKind;
use posix_errors::{PosixError, EINVAL, EINVALEXIT, ENODEV, ENXIO, EUTF8};
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
        UiError(match err {
            ErrorKind::IoError(io_err) => PosixError::from(io_err),
            ErrorKind::FmtError(e) => PosixError::new(135, e.to_string()),
            ErrorKind::Utf8Error(e) => PosixError::new(EUTF8, e.to_string()),
            ErrorKind::ParseIntError(e) => PosixError::new(EINVALEXIT + EINVAL, e.to_string()),
            ErrorKind::ResizingTerminalFailure(e) => PosixError::new(EINVALEXIT + ENXIO, e),
            ErrorKind::SettingTerminalTitleFailure => {
                PosixError::new(ENODEV, "Failed to set title".to_string())
            }
            e => PosixError::new(1, e.to_string()),
        })
    }
}

fn glv() -> Result<(), PosixError> {
    let app = arg_parser();

    let matches = app.get_matches();

    let working_dir = if let Some(wd) = matches.value_of("working_dir") {
        wd.to_string()
    } else {
        git_wrapper::top_level()?
    };

    let revision = matches.value_of("REVISION").unwrap();

    let paths = if let Some(p) = matches.values_of("path") {
        p.map(ToString::to_string).collect()
    } else {
        vec![]
    };
    if let Err(err) = run_ui(&working_dir, revision, paths) {
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
    working_dir: &str,
    revision: &str,
    paths: Vec<String>,
) -> Result<(), crossterm::ErrorKind> {
    let mut area = new_area();
    let (tx, rx) = mpsc::channel::<Event>();
    {
        thread::spawn(move || {
            while let Ok(event) = read() {
                if let Err(err) = tx.send(event) {
                    panic!("Error: {:?}", err)
                }
            }
        });
    }

    let mut drawable = build_drawable(&working_dir, revision, paths);
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
                    panic!("Something gone wrong: {:?}", err)
                }
            },
        }
    }

    shutdown_screen()?;
    Ok(())
}

fn arg_parser() -> App<'static> {
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
    app_from_crate!().arg(w_arg).arg(rev_arg).arg(paths_arg)
}

fn build_drawable(
    working_dir: &str,
    revision: &str,
    paths: Vec<String>,
) -> SplitLayout<TableWidget, DiffView, HistoryEntry> {
    let history_list = {
        let history_adapter = HistoryAdapter::new(&working_dir, revision, paths).unwrap();
        TableWidget::new(history_adapter)
    };
    let diff = DiffView::default();

    SplitLayout::new(history_list, diff)
}
