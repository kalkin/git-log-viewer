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
use std::sync::mpsc::{Receiver, SendError, Sender, TryRecvError};
use std::time::{SystemTime, UNIX_EPOCH};

use curl::easy::Easy;

use crate::cache;
use crate::commit::Oid;
use std::thread;
use tinyjson::JsonValue;
use url::Url;

use super::ActorThread;

pub struct GitHubRequest {
    pub oid: Oid,
    pub url: Url,
    pub pr_id: String,
}

pub struct GitHubResponse {
    pub oid: Oid,
    pub subject: String,
}

pub struct GitHubThread(ActorThread<GitHubRequest, GitHubResponse>);

impl GitHubThread {
    #[allow(clippy::too_many_lines)]
    pub(crate) fn new() -> Self {
        let (tx_1, receiver): (Sender<GitHubResponse>, Receiver<GitHubResponse>) = mpsc::channel();
        let (sender, rx_2): (Sender<GitHubRequest>, Receiver<GitHubRequest>) = mpsc::channel();
        let thread = thread::spawn(move || {
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

                let domain = v.url.domain().expect("Url with a domain name");
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
                let mut easy = Easy::new();
                easy.url(&url).unwrap();
                if let Some((response_code, headers, body)) = crate::utils::transfer(easy, domain) {
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

                    log::trace!("Response {} {}", response_code, url);

                    match response_code {
                        200 => {
                            if let Some(title) = Self::title_from_json(&body) {
                                log::debug!(
                                    "PR #{} (RL {})  ⇒ «{}»",
                                    pr_id,
                                    rate_limit_remaining,
                                    title
                                );

                                if let Err(err) = cache::store_api_response(
                                    &v.url,
                                    &format!("{}.json", pr_id),
                                    &body,
                                ) {
                                    log::warn!("PR #{}, {}", pr_id, err);
                                }
                                tx_1.send(GitHubResponse {
                                    oid,
                                    subject: format!("{} (#{})", title, pr_id),
                                })
                                .unwrap();
                            } else {
                                log::warn!("Got invalid JSON for #{}", pr_id);
                                log::debug!("{}", body);
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
            }
        });

        Self(ActorThread::new(thread, receiver, sender))
    }

    pub(crate) fn send(&self, req: GitHubRequest) -> Result<(), SendError<GitHubRequest>> {
        self.0.send(req)
    }

    pub(crate) fn try_recv(&self) -> Result<GitHubResponse, TryRecvError> {
        self.0.try_recv()
    }

    pub(crate) fn can_handle(url: &Url) -> bool {
        if let Some(domain) = url.domain() {
            return domain == "github.com";
        }
        false
    }

    pub fn from_cache(url: &Url, pr_id: &str) -> Option<String> {
        let json_data = match cache::fetch_api_response(url, &format!("{}.json", pr_id)) {
            Ok(v) => v,
            Err(err) => {
                log::warn!("PR #{}, {}", pr_id, err);
                None
            }
        }?;
        Self::title_from_json(&json_data)
    }

    fn title_from_json(body: &str) -> Option<String> {
        let json = body.parse::<JsonValue>().ok()?;
        if let JsonValue::String(title) = &json["title"] {
            return Some(title.to_string());
        }
        None
    }
}
