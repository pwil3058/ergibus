use std::{convert::From, ffi::OsString, io, path::PathBuf};

use crate::ReferencedContentData;
use failure::*;
use serde_json;
use serde_yaml;

/// A wrapper around the various error types than can be encountered
/// by this crate.
#[derive(Debug, Fail)]
pub enum RepoError {
    #[fail(display = "I/O Error")]
    IOError(#[cause] io::Error),
    #[fail(display = "Json Error")]
    JsonError(#[cause] serde_json::Error),
    #[fail(display = "Not implemented")]
    NotImplemented,
    #[fail(display = "{:?}: repository path already exists", _0)]
    RepoDirExists(PathBuf),
    #[fail(display = "{}: unknown hash algorithm", _0)]
    UnknownHashAlgorithm(String),
    #[fail(display = "{}: unknown content token", _0)]
    UnknownToken(String),
    #[fail(display = "Serde Yaml Error")]
    YamlError(#[cause] serde_yaml::Error),
    #[fail(display = "{:?}: malformed string", _0)]
    BadOsString(OsString),
    #[fail(display = "Still has {} references to {} itemts", _0, _1)]
    StillBeingReferenced(u128, u64),
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

impl From<OsString> for RepoError {
    fn from(os_string: OsString) -> Self {
        RepoError::BadOsString(os_string)
    }
}

impl From<ReferencedContentData> for RepoError {
    fn from(rcd: ReferencedContentData) -> Self {
        RepoError::StillBeingReferenced(rcd.num_references, rcd.num_items)
    }
}
