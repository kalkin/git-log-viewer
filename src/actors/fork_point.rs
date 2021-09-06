#![allow(clippy::module_name_repetitions)]
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::thread::JoinHandle;

use crate::commit::{Commit, Oid};
use git_wrapper::is_ancestor;
use std::fmt::{Debug, Formatter};
use std::sync::mpsc;
use std::thread;

pub enum ForkPointCalculation {
    Done(bool),
    InProgress,
}

pub struct ForkPointThread {
    _thread: JoinHandle<()>,
    receiver: Receiver<ForkPointResponse>,
    sender: Sender<ForkPointRequest>,
}

pub struct ForkPointRequest {
    pub first: Oid,
    pub second: Oid,
    pub working_dir: String,
}

impl Debug for ForkPointRequest {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut first = self.first.0.clone();
        let mut second = self.second.0.clone();
        first.truncate(8);
        second.truncate(8);
        f.debug_struct("ForkPointRequest")
            .field("oid", &first)
            .field("oid", &second)
            .finish()
    }
}

pub struct ForkPointResponse {
    pub oid: Oid,
    pub value: bool,
}

impl Debug for ForkPointResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut oid = self.oid.0.clone();
        oid.truncate(8);
        f.debug_tuple("ForkPointResponse")
            .field(&oid)
            .field(&self.value.to_string())
            .finish()
    }
}

impl ForkPointThread {
    #[must_use]
    pub fn is_fork_point(working_dir: &str, first: &Oid, second: &Oid) -> bool {
        is_ancestor(working_dir, &first.0, &second.0).expect("Execute merge-base --is-ancestor")
    }

    pub fn send(&self, req: ForkPointRequest) {
        if let Err(e) = self.sender.send(req) {
            eprintln!("Error {:?}", e);
        }
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn try_recv(&self) -> Result<ForkPointResponse, TryRecvError> {
        self.receiver.try_recv()
    }

    pub fn request_calculation(
        &self,
        t: &Commit,
        above_commit: Option<&Commit>,
        working_dir: &str,
    ) -> ForkPointCalculation {
        let mut fork_point_calc = ForkPointCalculation::Done(false);
        if let Some(c) = above_commit {
            fork_point_calc = if c.is_merge() && c.children()[0] != *t.id() {
                let first = t.id().clone();
                let second = c.children().first().expect("oid").clone();
                let request = ForkPointRequest {
                    first,
                    second,
                    working_dir: working_dir.to_string(),
                };
                self.send(request);
                ForkPointCalculation::InProgress
            } else {
                ForkPointCalculation::Done(false)
            }
        }
        fork_point_calc
    }

    pub fn new() -> Self {
        let (tx_1, rx_1): (Sender<ForkPointResponse>, Receiver<ForkPointResponse>) =
            mpsc::channel();
        let (tx_2, rx_2): (Sender<ForkPointRequest>, Receiver<ForkPointRequest>) = mpsc::channel();
        let child = thread::spawn(move || {
            while let Ok(v) = rx_2.recv() {
                let working_dir = v.working_dir.as_str();
                tx_1.send(ForkPointResponse {
                    oid: v.first.clone(),
                    value: ForkPointThread::is_fork_point(working_dir, &v.first, &v.second),
                })
                .unwrap();
            }
        });
        ForkPointThread {
            _thread: child,
            receiver: rx_1,
            sender: tx_2,
        }
    }
}
