use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::thread::JoinHandle;

use crate::commit::Oid;
use git_wrapper::is_ancestor;
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

pub struct ForkPointResponse {
    pub oid: Oid,
    pub value: bool,
}

impl ForkPointThread {
    pub fn is_fork_point(working_dir: &str, first: &Oid, second: &Oid) -> bool {
        is_ancestor(working_dir, &first.0, &second.0).expect("Execute merge-base --is-ancestor")
    }

    pub fn send(&self, req: ForkPointRequest) {
        if let Err(e) = self.sender.send(req) {
            log::error!("Error {:?}", e)
        }
    }

    pub fn try_recv(&self) -> Result<ForkPointResponse, TryRecvError> {
        self.receiver.try_recv()
    }
}

impl Default for ForkPointThread {
    fn default() -> Self {
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
