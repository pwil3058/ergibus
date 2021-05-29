use std::{convert::From, error, ffi::OsString, fmt, io, path::PathBuf};

use crate::ReferencedContentData;
use serde_json;
use serde_yaml;

/// A wrapper around the various error types than can be encountered
/// by this crate.
#[derive(Debug)]
pub enum RepoError {
    IOError(io::Error),
    JsonError(serde_json::Error),
    NotImplemented,
    RepoDirExists(PathBuf),
    UnknownHashAlgorithm(String),
    UnknownToken(String),
    YamlError(serde_yaml::Error),
    BadOsString(OsString),
    StillBeingReferenced(ReferencedContentData),
}

impl fmt::Display for RepoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use RepoError::*;
        match self {
            IOError(error) => write!(f, "{}", error),
            JsonError(error) => write!(f, "{}", error),
            NotImplemented => write!(f, "Feature not yet implemented"),
            RepoDirExists(path) => write!(f, "{:?}: repository path already exists", path),
            UnknownHashAlgorithm(string) => write!(f, "{}: unknown hash algorithm", string),
            UnknownToken(string) => write!(f, "{}: unknown content token", string),
            YamlError(error) => write!(f, "{}", error),
            BadOsString(os_string) => write!(f, "{:?}: malformed string", os_string),
            StillBeingReferenced(rcd) => write!(
                f,
                "Still has {} references to {} itemts",
                rcd.num_references, rcd.num_items
            ),
        }
    }
}

impl error::Error for RepoError {}

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

impl From<OsString> for RepoError {
    fn from(os_string: OsString) -> Self {
        RepoError::BadOsString(os_string)
    }
}
