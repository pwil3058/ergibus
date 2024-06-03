#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;

use path_ext;

pub mod archive;
pub mod attributes;
pub mod config;
pub mod fs_objects;
pub mod path_buf_ext;
mod report;
pub mod snapshot;

use crate::archive::ArchiveNameOrDirPath;

static UNEXPECTED: &str = "Unexpected error: please inform <pwil3058@bigpond.net.au>";

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
    ArchiveIncludePathError(path_ext::Error, std::path::PathBuf),

    GlobError(globset::Error),

    IOError(std::io::Error),

    ContentCopyIOError(std::io::Error),
    RepoError(dychatat_lib::RepoError),
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
    SnapshotUnknownFile(std::path::PathBuf),
    SnapshotUnknownDirectory(std::path::PathBuf),
    SnapshotWriteIOError(std::io::Error, std::path::PathBuf),
    SnapshotSerializeError(serde_json::Error),
    SnapshotsFailed(i32),

    DuplicateFileSystemObjectName,
    FSOMalformedPath(std::path::PathBuf),
    FSOBrokenSymLink(std::path::PathBuf, std::path::PathBuf),
}

impl From<dychatat_lib::RepoError> for Error {
    fn from(error: dychatat_lib::RepoError) -> Self {
        Error::RepoError(error)
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::IOError(error)
    }
}

pub type EResult<T> = Result<T, Error>;

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Ergibus library error: {:?}", self)
    }
}

impl std::error::Error for Error {}
