use git_stree::Subtrees;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::thread;
use std::thread::JoinHandle;

use git_stree::SubtreeConfig;

use crate::commit::Oid;

pub struct SubtreeChangesRequest {
    pub oid: Oid,
}
pub struct SubtreeChangesResponse {
    pub oid: Oid,
    pub subtrees: Vec<SubtreeConfig>,
}

pub struct SubtreeThread {
    _thread: JoinHandle<()>,
    receiver: Receiver<SubtreeChangesResponse>,
    sender: Sender<SubtreeChangesRequest>,
}

impl SubtreeThread {
    pub(crate) fn new(subtrees: Subtrees) -> Self {
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
                if let Ok(result) = subtrees.changed_modules(&v.oid.to_string()) {
                    tx_1.send(SubtreeChangesResponse {
                        oid: v.oid,
                        subtrees: result,
                    })
                    .unwrap();
                }
            }
        });
        SubtreeThread {
            _thread: child,
            receiver: rx_1,
            sender: tx_2,
        }
    }

    pub(crate) fn send(&self, req: SubtreeChangesRequest) {
        if let Err(e) = self.sender.send(req) {
            eprintln!("Error {:?}", e);
        }
    }

    pub(crate) fn try_recv(&self) -> Result<SubtreeChangesResponse, TryRecvError> {
        self.receiver.try_recv()
    }
}
