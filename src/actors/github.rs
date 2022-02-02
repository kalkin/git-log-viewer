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

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};

use std::collections::HashMap;

use curl::easy::Easy;

use crate::commit::Oid;
use std::thread;
use tinyjson::JsonValue;
use url::Url;

pub struct GitHubRequest {
    pub oid: Oid,
    pub url: Url,
    pub pr_id: String,
}

pub struct GitHubResponse {
    pub oid: Oid,
    pub subject: String,
}

pub struct GitHubThread {
    _thread: JoinHandle<()>,
    receiver: Receiver<GitHubResponse>,
    sender: Sender<GitHubRequest>,
}

impl GitHubThread {
    #[allow(clippy::too_many_lines)]
    pub(crate) fn new() -> Self {
        let (tx_1, rx_1): (Sender<GitHubResponse>, Receiver<GitHubResponse>) = mpsc::channel();
        let (tx_2, rx_2): (Sender<GitHubRequest>, Receiver<GitHubRequest>) = mpsc::channel();
        let child = thread::spawn(move || {
            let mut rate_limit_remaining = 60;
            let mut rate_limit_reset = u64::MAX;
            while let Ok(v) = rx_2.recv() {
                if !Self::can_handle(&v.url) {
                    log::debug!("Can not handle url {}", &v.url);
                    continue;
                }

                let pr_id = v.pr_id;
                if rate_limit_remaining == 0 {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    if now < rate_limit_reset {
                        let delta = rate_limit_reset - now;
                        log::info!(
                            "Skipping lookup #{} Rate limited for {} seconds",
                            pr_id,
                            delta
                        );
                        continue;
                    }
                }

                let mut segments = v.url.path_segments().unwrap();
                let owner = segments.next().unwrap();
                let repo = segments.next().unwrap();
                let oid = v.oid;
                log::debug!(
                    "Looking up PR #{} for {}/{}/{}",
                    pr_id,
                    owner,
                    repo,
                    &oid.0[0..7]
                );

                let url = format!(
                    "https://api.github.com/repos/{}/{}/pulls/{}",
                    owner, repo, pr_id
                );
                let mut headers: HashMap<String, String> = HashMap::with_capacity(25);
                let mut body: String = String::new();
                let mut easy = Easy::new();
                {
                    // Fetch data
                    easy.useragent("kalkin/glv").unwrap();
                    easy.url(&url).unwrap();
                    let mut transfer = easy.transfer();
                    transfer
                        .header_function(|line| {
                            let line = String::from_utf8_lossy(line);
                            let line = line.trim();
                            let tmp: Vec<_> = line.splitn(2, ": ").collect();
                            if tmp.len() == 2 {
                                let key = tmp[0].to_string();
                                let value = tmp[1].to_string();
                                headers.insert(key, value);
                            }
                            true
                        })
                        .unwrap();
                    transfer
                        .write_function(|data| {
                            // body = String::from_utf8(Vec::from(data)).unwrap();
                            body.push_str(String::from_utf8(Vec::from(data)).unwrap().as_str());
                            Ok(data.len())
                        })
                        .unwrap();

                    transfer.perform().unwrap();
                }

                {
                    // Check rate limiting headers
                    if let Some(value) = headers.get("X-RateLimit-Remaining") {
                        if let Ok(number) = value.parse::<u32>() {
                            log::trace!("RateLimit-Remaining: {}", number);
                            rate_limit_remaining = number;
                        }
                    }

                    if let Some(value) = headers.get("X-RateLimit-Reset") {
                        if let Ok(since_epoch) = value.parse::<u64>() {
                            let now = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_secs();
                            log::trace!("RateLimit-Reset in {} seconds", since_epoch - now);
                            rate_limit_reset = since_epoch;
                        }
                    }
                }

                let response_code = easy.response_code().unwrap();
                log::trace!("Response {} {}", response_code, url);

                match response_code {
                    200 => {
                        if let Ok(parsed) = body.parse::<JsonValue>() {
                            match &parsed["title"] {
                                JsonValue::String(title) => {
                                    log::debug!(
                                        "PR #{} (RL {})  ⇒ «{}»",
                                        pr_id,
                                        rate_limit_remaining,
                                        title
                                    );
                                    tx_1.send(GitHubResponse {
                                        oid,
                                        subject: format!("{} (#{})", title, pr_id),
                                    })
                                    .unwrap();
                                }
                                _ => {
                                    log::warn!(
                                        "PR #{}: Got unexpected {:?}",
                                        pr_id,
                                        parsed["title"]
                                    );
                                }
                            }
                        } else {
                            log::warn!("Got invalid JSON for PR #{}: {:?}", pr_id, body);
                        }
                    }
                    403 => {
                        log::warn!("We are asked to rate limit our selfs");
                        log::debug!("{}", body);
                        rate_limit_remaining = 0;
                    }
                    _ => {
                        log::warn!("Unexpected API Response {}", response_code);
                        log::debug!("{}", body);
                    }
                }
            }
        });

        Self {
            _thread: child,
            receiver: rx_1,
            sender: tx_2,
        }
    }

    pub(crate) fn send(&self, req: GitHubRequest) {
        if let Err(e) = self.sender.send(req) {
            panic!("Error {:?}", e);
        }
    }

    pub(crate) fn try_recv(&self) -> Result<GitHubResponse, TryRecvError> {
        self.receiver.try_recv()
    }

    pub(crate) fn can_handle(url: &Url) -> bool {
        if let Some(domain) = url.domain() {
            return domain == "github.com";
        }
        false
    }
}
