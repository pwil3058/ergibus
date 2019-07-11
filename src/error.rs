use std::{convert::From, io, path::PathBuf};

use serde_json;
use serde_yaml;

#[derive(Debug)]
pub enum RepoError {
    IOError(io::Error),
    JsonError(serde_json::Error),
    NotImplemented,
    RepoDirExists(PathBuf),
    UnknownHashAlgorithm(String),
    UnknownToken(String),
    YamlError(serde_yaml::Error),
}

impl From<io::Error> for RepoError {
    fn from(error: io::Error) -> Self {
        RepoError::IOError(error)
    }
}

impl From<serde_json::Error> for RepoError {
    fn from(error: serde_json::Error) -> Self {
        RepoError::JsonError(error)
    }
}

impl From<serde_yaml::Error> for RepoError {
    fn from(error: serde_yaml::Error) -> Self {
        RepoError::YamlError(error)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
