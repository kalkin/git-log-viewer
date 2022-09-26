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

use std::{
    sync::mpsc::{Receiver, SendError, Sender, TryRecvError},
    thread::JoinHandle,
};

pub mod bitbucket;
pub mod fork_point;
pub mod github;
pub mod subtrees;

struct ActorThread<Request, Response> {
    _thread: JoinHandle<()>,
    receiver: Receiver<Response>,
    sender: Sender<Request>,
}

impl<Request, Response> ActorThread<Request, Response> {
    fn new(thread: JoinHandle<()>, receiver: Receiver<Response>, sender: Sender<Request>) -> Self {
        Self {
            _thread: thread,
            receiver,
            sender,
        }
    }

    fn send(&self, request: Request) -> Result<(), SendError<Request>> {
        self.sender.send(request)
    }

    fn try_recv(&self) -> Result<Response, TryRecvError> {
        self.receiver.try_recv()
    }
}
