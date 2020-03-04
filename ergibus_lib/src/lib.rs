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

#[derive(Debug)]
pub enum Error {
    ArchiveDirError(std::io::Error, std::path::PathBuf),
    ArchiveEmpty(crate::snapshot::ArchiveOrDirPath),
    ArchiveExists(String),
    ArchiveReadError(std::io::Error, std::path::PathBuf),
    ArchiveWriteError(std::io::Error, std::path::PathBuf),
    ArchiveYamlReadError(serde_yaml::Error, String),
    ArchiveYamlWriteError(serde_yaml::Error, String),
    RelativeIncludePath(std::path::PathBuf, String),

    GlobError(globset::Error),

    ContentCopyIOError(std::io::Error),
    RepoError(dychatat::RepoError),
    RepoExists(String),
    RepoReadError(std::io::Error, std::path::PathBuf),
    RepoWriteError(std::io::Error, std::path::PathBuf),
    UnknownRepo(String),

    LastSnapshot(crate::snapshot::ArchiveOrDirPath),
    NoSnapshotAvailable,
    SnapshotDeleteIOError(std::io::Error, std::path::PathBuf),
    SnapshotDirIOError(std::io::Error, std::path::PathBuf),
    SnapshotIndexOutOfRange(crate::snapshot::ArchiveOrDirPath, i64),
    SnapshotMismatch(std::path::PathBuf),
    SnapshotMismatchDirty(std::io::Error, std::path::PathBuf),
    SnapshotMoveAsideFailed(std::path::PathBuf, std::io::Error),
    SnapshotReadIOError(std::io::Error, std::path::PathBuf),
    SnapshotReadJsonError(serde_json::Error, std::path::PathBuf),
    SnapshotUnknownFile(String, String, std::path::PathBuf),
    SnapshotUnknownDirectory(String, String, std::path::PathBuf),
    SnapshotWriteIOError(std::io::Error, std::path::PathBuf),
    SnapshotSerializeError(serde_json::Error),
}

impl From<dychatat::RepoError> for Error {
    fn from(error: dychatat::RepoError) -> Self {
        Error::RepoError(error)
    }
}

pub type EResult<T> = Result<T, Error>;
