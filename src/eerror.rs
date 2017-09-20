// Copyright 2017 Peter Williams <pwil3058@gmail.com>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::io;
use std::path::PathBuf;

use globset;
use serde_json;
use serde_yaml;

#[derive(Debug)]
pub enum EError {
    ArchiveGlobError(globset::Error, String),
    ArchiveExists(String),
    GlobError(globset::Error),
    ArchiveReadError(io::Error, PathBuf),
    ArchiveWriteError(io::Error, PathBuf),
    ArchiveDirError(io::Error, PathBuf),
    RelativeIncludePath(PathBuf, String),
    ArchiveYamlReadError(serde_yaml::Error, String),
    ArchiveYamlWriteError(serde_yaml::Error, String),

    RepoExists(String),
    UnknownRepo(String),
    UnknownContentKey(String),
    UnknownKeyAlgorithm(String),
    ContentStoreIOError(io::Error),
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
    SnapshotWriteIOError(io::Error, PathBuf),
    SnapshotReadIOError(io::Error, PathBuf),
    SnapshotDeleteIOError(io::Error, PathBuf),
    SnapshotReadJsonError(serde_json::Error, PathBuf),
    SnapshotMismatch(PathBuf),
    SnapshotMismatchDirty(io::Error, PathBuf),
    SnapshotSerializeError(serde_json::Error),
}

pub type EResult<T> = Result<T, EError>;
