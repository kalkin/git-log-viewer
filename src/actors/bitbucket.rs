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
use std::thread;
use std::thread::JoinHandle;

use curl::easy::Easy;
use tinyjson::JsonValue;
use url::Url;

use crate::commit::Oid;
use crate::credentials;

#[allow(clippy::module_name_repetitions)]
pub struct BitbucketRequest {
    pub oid: Oid,
    pub url: Url,
    pub pr_id: String,
}

#[allow(clippy::module_name_repetitions)]
pub struct BitbucketResponse {
    pub oid: Oid,
    pub subject: String,
}

#[allow(clippy::module_name_repetitions)]
pub struct BitbucketThread {
    _thread: JoinHandle<()>,
    receiver: Receiver<BitbucketResponse>,
    sender: Sender<BitbucketRequest>,
}

fn api_url(v: &BitbucketRequest) -> Option<Url> {
    let domain = v
        .url
        .domain()
        .expect("At this point we should have a domain");
    let tmp: Vec<&str> = v.url.path_segments().unwrap().collect();
    if tmp.len() >= 2 {
        let [workspace, repo_slug] = [tmp[0], tmp[1]];
        let text = format!(
            "https://{}/rest/api/1.0/projects/{}/repos/{}/pull-requests/{}",
            domain, workspace, repo_slug, v.pr_id
        );
        return Url::parse(&text).ok();
    }
    None
}

impl BitbucketThread {
    pub(crate) fn new() -> Self {
        let (tx_1, rx_1): (Sender<BitbucketResponse>, Receiver<BitbucketResponse>) =
            mpsc::channel();
        let (tx_2, rx_2): (Sender<BitbucketRequest>, Receiver<BitbucketRequest>) = mpsc::channel();
        let child = thread::spawn(move || {
            let mut stopped = false;
            while let Ok(v) = rx_2.recv() {
                if stopped {
                    log::debug!("Stopped. Skipping #{}", v.pr_id);
                    continue;
                }

                if !Self::can_handle(&v.url) {
                    log::debug!("Can not handle url {}", &v.url);
                    continue;
                }

                let url = if let Some(url) = api_url(&v) {
                    url
                } else {
                    log::warn!("Failed to parse BitBucket Server url from: {:?}", v.url);
                    continue;
                };

                let pr_id = v.pr_id;
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

                let mut easy = Easy::new();
                easy.url(url.as_str()).unwrap();
                if let Some((response_code, _headers, body)) = crate::utils::transfer(easy) {
                    match response_code {
                        200 => {
                            if let Ok(parsed) = body.parse::<JsonValue>() {
                                match &parsed["title"] {
                                    JsonValue::String(title) => {
                                        log::debug!("PR #{} ⇒ {}", pr_id, title);
                                        tx_1.send(BitbucketResponse {
                                            oid,
                                            subject: format!("{} (#{})", title, pr_id),
                                        })
                                        .unwrap();
                                    }
                                    _ => {
                                        log::error!(
                                            "PR #{}: Got unexpected {:?}",
                                            pr_id,
                                            parsed["title"]
                                        );
                                    }
                                }
                            } else {
                                log::error!("Got invalid JSON for #{}", pr_id);
                                log::debug!("{}", body);
                            }
                        }
                        404 => {
                            log::info!("PR #{} not found on {}", pr_id, url.domain().unwrap());
                            log::trace!("Url API tried: {}", url);
                        }
                        401 => {
                            log::error!("Authentication to {} failed", url.domain().unwrap());
                            stopped = true;
                        }
                        _ => {
                            log::error!("Unexpected API Response {}", response_code);
                            log::debug!("{}", body);
                        }
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

    pub(crate) fn send(&self, req: BitbucketRequest) {
        if let Err(e) = self.sender.send(req) {
            panic!("Error {:?}", e);
        }
    }

    pub(crate) fn try_recv(&self) -> Result<BitbucketResponse, TryRecvError> {
        self.receiver.try_recv()
    }

    pub(crate) fn can_handle(url: &Url) -> bool {
        if let Some(domain) = url.domain() {
            // TODO proper recognition via http api call
            return domain.contains("bitbucket");
        }
        false
    }
}
