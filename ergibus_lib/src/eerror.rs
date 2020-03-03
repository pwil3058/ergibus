use std::io;
use std::path::PathBuf;

use globset;
use serde_json;
use serde_yaml;

use crate::snapshot::ArchiveOrDirPath;

#[derive(Debug)]
pub enum EError {
    ArchiveGlobError(globset::Error, String),
    ArchiveEmpty(ArchiveOrDirPath),
    ArchiveExists(String),
    GlobError(globset::Error),
    ArchiveReadError(io::Error, PathBuf),
    ArchiveWriteError(io::Error, PathBuf),
    ArchiveDirError(io::Error, PathBuf),
    RelativeIncludePath(PathBuf, String),
    ArchiveYamlReadError(serde_yaml::Error, String),
    ArchiveYamlWriteError(serde_yaml::Error, String),

    RepoError(dychatat::RepoError),

    RepoExists(String),
    UnknownRepo(String),
    UnknownContentKey(String),
    UnknownKeyAlgorithm(String),
    ContentStoreIOError(io::Error),
    ContentReadIOError(io::Error),
    ContentCopyIOError(io::Error),
    RepoCreateError(io::Error, PathBuf),
    RepoReadError(io::Error, PathBuf),
    RepoWriteError(io::Error, PathBuf),
    RepoYamlWriteError(serde_yaml::Error, String),
    RepoYamlReadError(serde_yaml::Error, String),
    RefCounterReadIOError(io::Error),
    RefCounterWriteIOError(io::Error),
    RefCounterReadJsonError(serde_json::Error),
    RefCounterSerializeError(serde_json::Error),

    NoSnapshotAvailable,
    LastSnapshot(ArchiveOrDirPath),
    SnapshotIndexOutOfRange(ArchiveOrDirPath, i64),
    SnapshotUnknownFile(String, String, PathBuf),
    SnapshotUnknownDirectory(String, String, PathBuf),
    SnapshotMoveAsideFailed(PathBuf, io::Error),
    SnapshotDirIOError(io::Error, PathBuf),
    SnapshotWriteIOError(io::Error, PathBuf),
    SnapshotReadIOError(io::Error, PathBuf),
    SnapshotDeleteIOError(io::Error, PathBuf),
    SnapshotReadJsonError(serde_json::Error, PathBuf),
    SnapshotMismatch(PathBuf),
    SnapshotMismatchDirty(io::Error, PathBuf),
    SnapshotSerializeError(serde_json::Error),
}

impl From<dychatat::RepoError> for EError {
    fn from(error: dychatat::RepoError) -> Self {
        EError::RepoError(error)
    }
}

pub type EResult<T> = Result<T, EError>;

// TODO: implement std::error::Error and std::fmt::Display for EError
