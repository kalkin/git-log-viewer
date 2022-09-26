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
use std::sync::mpsc::{self, SendError};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::thread;

use git_stree::SubtreeConfig;

use crate::commit::Oid;

use super::ActorThread;

pub struct SubtreeChangesRequest {
    pub oid: Oid,
}
pub struct SubtreeChangesResponse {
    pub oid: Oid,
    pub subtrees: Vec<SubtreeConfig>,
}

pub struct SubtreeThread(ActorThread<SubtreeChangesRequest, SubtreeChangesResponse>);

impl SubtreeThread {
    pub(crate) fn new(subtrees: Subtrees) -> Self {
        let (tx_1, receiver): (
            Sender<SubtreeChangesResponse>,
            Receiver<SubtreeChangesResponse>,
        ) = mpsc::channel();
        let (sender, rx_2): (
            Sender<SubtreeChangesRequest>,
            Receiver<SubtreeChangesRequest>,
        ) = mpsc::channel();

        let thread = thread::spawn(move || {
            while let Ok(v) = rx_2.recv() {
                if let Ok(result) = subtrees.changed_modules(&v.oid.to_string()) {
                    tx_1.send(SubtreeChangesResponse {
                        oid: v.oid,
                        subtrees: result,
                    })
                    .expect("Send SubtreeChangesResponse");
                }
            }
        });
        Self(ActorThread::new(thread, receiver, sender))
    }

    pub fn send(
        &self,
        request: SubtreeChangesRequest,
    ) -> Result<(), SendError<SubtreeChangesRequest>> {
        self.0.send(request)
    }

    pub fn try_recv(&self) -> Result<SubtreeChangesResponse, TryRecvError> {
        self.0.try_recv()
    }
}
