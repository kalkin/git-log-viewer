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
        Self {
            _thread: child,
            receiver: rx_1,
            sender: tx_2,
        }
    }

    pub(crate) fn send(&self, req: SubtreeChangesRequest) {
        if let Err(e) = self.sender.send(req) {
            log::error!("Error {:?}", e);
        }
    }

    pub(crate) fn try_recv(&self) -> Result<SubtreeChangesResponse, TryRecvError> {
        self.receiver.try_recv()
    }
}
