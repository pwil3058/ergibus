use std::{convert::From, ffi::OsString, io, path::PathBuf};

use crate::ReferencedContentData;
use serde_json;
use serde_yaml;
use thiserror::*;

/// A wrapper around the various error types than can be encountered
/// by this crate.
#[derive(Debug, Error)]
pub enum RepoError {
    #[error("I/O Error")]
    IOError(#[from] io::Error),
    #[error("Json Error")]
    JsonError(#[from] serde_json::Error),
    #[error("Not implemented")]
    NotImplemented,
    #[error("{0:?}: a repository with that name already exists")]
    RepoExists(String),
    #[error("{0:?}: repository path already exists")]
    RepoDirExists(PathBuf),
    #[error("{0:?}: no repository with that name exists")]
    UnknownRepo(String),
    #[error("{0}: unknown hash algorithm")]
    UnknownHashAlgorithm(String),
    #[error("{0}: unknown content token")]
    UnknownToken(String),
    #[error("Serde Yaml Error")]
    YamlError(#[from] serde_yaml::Error),
    #[error("{0:?}: malformed string")]
    BadOsString(OsString),
    #[error("Still has {0} references to {1} items")]
    StillBeingReferenced(u128, u64),
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

pub type RepoResult<T> = Result<T, RepoError>;
