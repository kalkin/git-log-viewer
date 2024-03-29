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
use std::thread;

use curl::easy::Easy;
use tinyjson::JsonValue;
use url::Url;

use crate::cache;
use crate::commit::Oid;

use super::ActorThread;

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
pub struct BitbucketThread(ActorThread<BitbucketRequest, BitbucketResponse>);

fn api_url(v: &BitbucketRequest) -> Option<Url> {
    let domain = v
        .url
        .domain()
        .expect("At this point we should have a domain");
    let split = v.url.path_segments();
    let tmp: Vec<&str> = split.map(Iterator::collect).unwrap_or_default();
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
        let (tx_1, receiver): (Sender<BitbucketResponse>, Receiver<BitbucketResponse>) =
            mpsc::channel();
        let (sender, rx_2): (Sender<BitbucketRequest>, Receiver<BitbucketRequest>) =
            mpsc::channel();
        let thread = thread::spawn(move || {
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
                if let Some((response_code, _headers, body)) =
                    crate::utils::transfer(easy, v.url.domain().unwrap())
                {
                    match response_code {
                        200 => {
                            if let Some(title) = Self::title_from_json(&body) {
                                log::debug!("PR #{} ⇒ {}", pr_id, title);
                                if let Err(err) = cache::store_api_response(
                                    &v.url,
                                    &format!("{}.json", pr_id),
                                    &body,
                                ) {
                                    log::warn!("PR #{}, {}", pr_id, err);
                                }
                                tx_1.send(BitbucketResponse {
                                    oid,
                                    subject: format!("{} (#{})", title, pr_id),
                                })
                                .unwrap();
                            } else {
                                log::warn!("Got invalid JSON for #{}", pr_id);
                                log::debug!("{}", body);
                            }
                        }
                        404 => {
                            log::info!("PR #{} not found on {:?}", pr_id, url.domain());
                            log::trace!("Url API tried: {}", url);
                        }
                        401 => {
                            log::error!("Authentication to {:?} failed", url.domain());
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

        Self(ActorThread::new(thread, receiver, sender))
    }

    pub(crate) fn send(&self, req: BitbucketRequest) -> Result<(), SendError<BitbucketRequest>> {
        self.0.send(req)
    }

    pub(crate) fn try_recv(&self) -> Result<BitbucketResponse, TryRecvError> {
        self.0.try_recv()
    }

    pub(crate) fn can_handle(url: &Url) -> bool {
        if let Some(domain) = url.domain() {
            // TODO proper recognition via http api call
            return domain.contains("bitbucket");
        }
        false
    }

    fn title_from_json(body: &str) -> Option<String> {
        let json = body.parse::<JsonValue>().ok()?;
        if let JsonValue::String(title) = &json["title"] {
            return Some(title.to_string());
        }
        None
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
}
