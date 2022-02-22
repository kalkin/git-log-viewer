use directories::BaseDirs;
use netrc::{Host, Netrc};
use std::fs::File;
use std::io::BufReader;

pub fn token(domain_name: &str) -> Option<(String, Option<String>)> {
    let base_dirs = BaseDirs::new()?;
    let path = base_dirs.home_dir().join(".netrc");
    let f = File::open(path).ok()?;
    let buf = BufReader::new(f);
    let netrc = Netrc::parse(buf).unwrap();
    let results: Vec<&Host> = netrc.hosts.iter().filter(|h| h.0 == domain_name).collect();
    if !results.is_empty() {
        let username = results[0].1.login.clone();
        let password = results[0].1.password.clone();
        return Some((username, password));
    }
    None
}
