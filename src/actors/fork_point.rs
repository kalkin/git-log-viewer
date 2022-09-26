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

#![allow(clippy::module_name_repetitions)]
use std::sync::mpsc::{Receiver, Sender, TryRecvError};

use crate::commit::{Commit, Oid};
use std::fmt::{Debug, Formatter};
use std::sync::mpsc;
use std::thread;

use git_wrapper::Repository;

use super::ActorThread;

#[derive(Debug)]
pub enum ForkPointCalculation {
    Done(bool),
    InProgress,
}

pub struct ForkPointThread(ActorThread<ForkPointRequest, ForkPointResponse>);

pub struct ForkPointRequest {
    pub first: Oid,
    pub second: Oid,
}

#[cfg(not(tarpaulin_include))]
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
    pub first: Oid,
    pub second: Oid,
    pub value: bool,
}

#[cfg(not(tarpaulin_include))]
impl Debug for ForkPointResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut oid = self.first.0.clone();
        oid.truncate(8);
        f.debug_tuple("ForkPointResponse")
            .field(&oid)
            .field(&self.value.to_string())
            .finish()
    }
}

impl ForkPointThread {
    #[allow(clippy::missing_errors_doc)]
    pub fn try_recv(&self) -> Result<ForkPointResponse, TryRecvError> {
        self.0.try_recv()
    }

    pub fn request_calculation(
        &self,
        t: &Commit,
        above_commit: Option<&Commit>,
    ) -> ForkPointCalculation {
        let mut fork_point_calc = ForkPointCalculation::Done(false);
        if let Some(c) = above_commit {
            fork_point_calc = if c.is_merge() && c.parents()[1] != *t.id() {
                self.0
                    .send(ForkPointRequest {
                        first: t.id().clone(),
                        second: c.parents()[1].clone(),
                    })
                    .unwrap();
                ForkPointCalculation::InProgress
            } else {
                ForkPointCalculation::Done(false)
            }
        }
        fork_point_calc
    }

    pub fn new(repo: Repository) -> Self {
        let (tx_1, receiver): (Sender<ForkPointResponse>, Receiver<ForkPointResponse>) =
            mpsc::channel();
        let (sender, rx_2): (Sender<ForkPointRequest>, Receiver<ForkPointRequest>) =
            mpsc::channel();
        let thread = thread::spawn(move || {
            while let Ok(v) = rx_2.recv() {
                let value = repo.is_ancestor(&v.first.0, &v.second.0);
                tx_1.send(ForkPointResponse {
                    first: v.first.clone(),
                    second: v.second.clone(),
                    value,
                })
                .expect("Send ForkPointResponse");
            }
        });
        Self(ActorThread::new(thread, receiver, sender))
    }
}
