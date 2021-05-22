#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;

pub mod archive;
pub mod attributes;
pub mod config;
pub mod content;
//pub mod eerror;
mod path_buf_ext;
mod report;
pub mod snapshot;

use crate::archive::ArchiveNameOrDirPath;

#[derive(Debug)]
pub enum Error {
    ArchiveDirError(std::io::Error, std::path::PathBuf),
    ArchiveEmpty(ArchiveNameOrDirPath),
    ArchiveExists(String),
    ArchiveUnknown(String),
    ArchiveReadError(std::io::Error, std::path::PathBuf),
    ArchiveWriteError(std::io::Error, std::path::PathBuf),
    ArchiveYamlReadError(serde_yaml::Error, String),
    ArchiveYamlWriteError(serde_yaml::Error, String),
    RelativeIncludePath(std::path::PathBuf, String),

    GlobError(globset::Error),

    IOError(std::io::Error),

    ContentCopyIOError(std::io::Error),
    RepoError(dychatat::RepoError),
    RepoExists(String),
    RepoReadError(std::io::Error, std::path::PathBuf),
    RepoWriteError(std::io::Error, std::path::PathBuf),
    UnknownRepo(String),

    LastSnapshot(ArchiveNameOrDirPath),
    NoSnapshotAvailable,
    SnapshotDeleteIOError(std::io::Error, std::path::PathBuf),
    SnapshotDirIOError(std::io::Error, std::path::PathBuf),
    SnapshotIndexOutOfRange(ArchiveNameOrDirPath, i64),
    SnapshotMismatch(std::path::PathBuf),
    SnapshotMismatchDirty(std::io::Error, std::path::PathBuf),
    SnapshotMoveAsideFailed(std::path::PathBuf, std::io::Error),
    SnapshotReadIOError(std::io::Error, std::path::PathBuf),
    SnapshotReadJsonError(serde_json::Error, std::path::PathBuf),
    SnapshotUnknownFile(String, String, std::path::PathBuf),
    SnapshotUnknownDirectory(String, String, std::path::PathBuf),
    SnapshotWriteIOError(std::io::Error, std::path::PathBuf),
    SnapshotSerializeError(serde_json::Error),
    SnapshotUnknownContent(std::path::PathBuf),
    SnapshotsFailed(i32),
}

impl From<dychatat::RepoError> for Error {
    fn from(error: dychatat::RepoError) -> Self {
        Error::RepoError(error)
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::IOError(error)
    }
}

pub type EResult<T> = Result<T, Error>;
