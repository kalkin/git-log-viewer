use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::thread::JoinHandle;

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
    pub(crate) fn new() -> Self {
        let (tx_1, rx_1): (Sender<GitHubResponse>, Receiver<GitHubResponse>) = mpsc::channel();
        let (tx_2, rx_2): (Sender<GitHubRequest>, Receiver<GitHubRequest>) = mpsc::channel();
        let child = thread::spawn(move || {
            while let Ok(v) = rx_2.recv() {
                if !GitHubThread::can_handle(&v.url) {
                    continue;
                }

                let mut segments = v.url.path_segments().unwrap();
                let owner = segments.next().unwrap();
                let repo = segments.next().unwrap();
                let oid = v.oid;
                let pr_id = v.pr_id;

                let mut easy = Easy::new();
                let url = format!(
                    "https://api.github.com/repos/{}/{}/pulls/{}",
                    owner, repo, pr_id
                );
                easy.useragent("kalkin/glv").unwrap();
                easy.url(&url).unwrap();
                let mut body: String = String::new();
                {
                    let mut transfer = easy.transfer();
                    transfer
                        .write_function(|data| {
                            // body = String::from_utf8(Vec::from(data)).unwrap();
                            body.push_str(String::from_utf8(Vec::from(data)).unwrap().as_str());
                            Ok(data.len())
                        })
                        .unwrap();

                    transfer.perform().unwrap();
                }
                let response_code = easy.response_code().unwrap();
                if response_code != 200 {
                    continue;
                }
                if let Ok(parsed) = body.parse::<JsonValue>() {
                    match &parsed["title"] {
                        JsonValue::String(title) => {
                            tx_1.send(GitHubResponse {
                                oid,
                                subject: format!("{} (#{})", title, pr_id),
                            })
                            .unwrap();
                        }
                        _ => {
                            panic!("PR #{}: Got unexpected {:?}", pr_id, parsed["title"]);
                        }
                    }
                } else {
                    panic!("INVALID JSON for PR #{}: {:?}", pr_id, body);
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
        if let (Some(domain), Some(path_segments)) = (url.domain(), url.path_segments()) {
            let segs: Vec<&str> = path_segments.collect();
            return domain == "github.com" && segs.len() == 2;
        }
        false
    }
}
