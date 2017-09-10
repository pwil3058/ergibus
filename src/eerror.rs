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
pub enum AError {
    GlobError(globset::Error),
    IOError(io::Error, PathBuf),
    RelativeIncludePath(PathBuf),
    ContentError(CError),
    YamlError(serde_yaml::Error),
}

#[derive(Debug)]
pub enum CError {
    UnknownRepo(String),
    IOError(io::Error, PathBuf),
    UnknownToken(String),
    FileSystemError(io::Error),
}

#[derive(Debug)]
pub enum SSError {
    NoSnapshotAvailable,
    SnapshotMismatch,
    SnapshotMismatchDirty(io::Error),
    IOError(io::Error),
    JsonError(serde_json::Error),
    SnapshotReadIOError(io::Error),
    SnapshotReadJsonError(serde_json::Error),
    ArchiveError(AError),
    ContentError(CError),
}
