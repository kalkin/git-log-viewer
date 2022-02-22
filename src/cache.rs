use std::path::{Path, PathBuf};

use url::Url;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Provided absolute path {0}")]
    AbsolutePath(PathBuf),
    #[error("Url has no domain")]
    NoDomain(Url),
    #[error("Failed to find application cache_path")]
    CacheFailure,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

fn store(path: std::path::PathBuf, body: &str) -> Result<(), Error> {
    if !path.is_relative() {
        return Err(Error::AbsolutePath(path));
    }

    if let Some(proj_dirs) = directories::ProjectDirs::from("", "", "glv") {
        let cache_path = proj_dirs.cache_dir().join(path);
        std::fs::create_dir_all(cache_path.parent().expect("Parent directory"))?;
        Ok(std::fs::write(cache_path, body)?)
    } else {
        Err(Error::CacheFailure)
    }
}

fn fetch(path: std::path::PathBuf) -> Result<Option<String>, Error> {
    if !path.is_relative() {
        return Err(Error::AbsolutePath(path));
    }

    let proj_dirs = directories::ProjectDirs::from("", "", "glv").expect("");
    let cache_path = proj_dirs.cache_dir().join(path);
    if cache_path.exists() {
        Ok(Some(std::fs::read_to_string(cache_path)?))
    } else {
        Ok(None)
    }
}

fn path_from_url(url: &Url) -> Result<PathBuf, Error> {
    let domain = url.domain().ok_or_else(|| Error::NoDomain(url.clone()))?;
    let path = format!("{}{}", domain, url.path());
    Ok(Path::new(&path).to_path_buf())
}

#[test]
fn test_path_from_url() {
    let url = url::Url::parse("https://github.com/kalkin/bar").unwrap();
    let result = path_from_url(&url).unwrap();
    let actual = result.to_str().unwrap();
    let expected = "github.com/kalkin/bar";
    assert_eq!(expected, actual);
}

pub fn store_api_response(url: &url::Url, id: &str, body: &str) -> Result<(), Error> {
    let path = path_from_url(url)?.join(id);
    store(path, body)
}

pub fn fetch_api_response(url: &url::Url, id: &str) -> Result<Option<String>, Error> {
    let path = path_from_url(url)?.join(id);
    fetch(path)
}
