use crate::{commit::parse_remote_url, credentials};
use curl::easy::Easy;
use git_wrapper::Remote;
use std::collections::HashMap;
use url::Url;

pub fn transfer(mut easy: Easy, domain: &str) -> Option<(u32, HashMap<String, String>, String)> {
    if let Some((username, maybe_pw)) = credentials::token(domain) {
        if let Some(password) = maybe_pw {
            log::trace!("Authentication via username {} for {}", username, domain);
            easy.username(&username).ok()?;
            easy.password(&password).ok()?;
        } else {
            log::trace!("Authentication via token for {}", domain);
            let mut headers = curl::easy::List::new();
            let token = username;
            headers
                .append(&format!("Authorization: Bearer {}", token))
                .ok()?;
            easy.http_headers(headers).ok()?;
        }
    } else {
        log::trace!("No authentication for {}", domain);
    }
    let mut headers: HashMap<String, String> = HashMap::with_capacity(25);
    let mut body: String = String::new();
    let transfer_result = {
        // Fetch data
        easy.useragent("kalkin/glv").ok()?;
        let mut transfer = easy.transfer();
        #[allow(clippy::shadow_reuse)]
        transfer
            .header_function(|line| {
                let line = String::from_utf8_lossy(line);
                let line = line.trim();
                let tmp: Vec<_> = line.splitn(2, ": ").collect();
                if tmp.len() == 2 {
                    let key = tmp[0].to_owned();
                    let value = tmp[1].to_owned();
                    headers.insert(key, value);
                }
                true
            })
            .ok()?;
        transfer
            .write_function(|data| {
                body.push_str(&String::from_utf8_lossy(&Vec::from(data)));
                Ok(data.len())
            })
            .ok()?;

        transfer.perform()
    };
    if let Err(e) = transfer_result {
        log::error!("{:?}", e);
        return None;
    }
    let response_code = easy.response_code().ok()?;
    Some((response_code, headers, body))
}

pub fn find_forge_url(hash_map: &HashMap<String, Remote>) -> Option<Url> {
    if let Some(remote) = hash_map.get("origin") {
        if let Some(s) = &remote.fetch {
            if let Some(u) = parse_remote_url(s) {
                return Some(u);
            }
        }
        if let Some(s) = &remote.push {
            if let Some(u) = parse_remote_url(s) {
                return Some(u);
            }
        }
    }
    for r in hash_map.values() {
        if let Some(s) = &r.fetch {
            if let Some(u) = parse_remote_url(s) {
                return Some(u);
            }
        }
        if let Some(s) = &r.push {
            if let Some(u) = parse_remote_url(s) {
                return Some(u);
            }
        }
    }
    None
}
