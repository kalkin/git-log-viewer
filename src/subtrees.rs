use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::thread;
use std::thread::JoinHandle;

use git_subtrees_improved::{changed_modules, SubtreeConfig};

use crate::core::Oid;

pub struct SubtreeChangesRequest {
    pub oid: Oid,
}
pub struct SubtreeChangesResponse {
    pub oid: Oid,
    pub subtrees: Vec<String>,
}

pub struct SubtreesThread {
    thread: JoinHandle<()>,
    receiver: Receiver<SubtreeChangesResponse>,
    sender: Sender<SubtreeChangesRequest>,
}

impl SubtreesThread {
    pub(crate) fn new(working_dir: String, all_subtrees: Vec<SubtreeConfig>) -> Self {
        let (tx_1, rx_1): (
            Sender<SubtreeChangesResponse>,
            Receiver<SubtreeChangesResponse>,
        ) = mpsc::channel();
        let (tx_2, rx_2): (
            Sender<SubtreeChangesRequest>,
            Receiver<SubtreeChangesRequest>,
        ) = mpsc::channel();

        let child = thread::spawn(move || {
            while let Ok(v) = rx_2.recv() {
                let result = changed_modules(&working_dir, &v.oid.to_string(), &all_subtrees);
                tx_1.send(SubtreeChangesResponse {
                    oid: v.oid,
                    subtrees: result,
                })
                .unwrap();
            }
        });
        SubtreesThread {
            thread: child,
            receiver: rx_1,
            sender: tx_2,
        }
    }

    pub(crate) fn send(&self, req: SubtreeChangesRequest) {
        self.sender.send(req);
    }

    pub(crate) fn try_recv(&self) -> Result<SubtreeChangesResponse, TryRecvError> {
        self.receiver.try_recv()
    }
}
