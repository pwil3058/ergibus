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

use std::error::Error;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf, Component};

pub fn ignore_report_or_crash(err: &io::Error, path: &Path) {
    if err.kind() != ErrorKind::NotFound {
        // we assume that "not found" is due to a race condition and don't report it
        if err.kind() == ErrorKind::PermissionDenied {
            // benign so just report it
            println!("{:?}: permission denied", path);
        } else {
            panic!("{:?}: {:?}: {:?}", err.kind(), err.description(), path);
        }
    }
}

pub fn report_broken_link_or_crash(err: &io::Error, link_path: &Path, target_path: &Path) {
    if err.kind() == ErrorKind::NotFound {
        println!("{:?} -> {:?}: broken symbolic link ignored", link_path, target_path);
    } else {
        panic!("{:?}: {:?}: {:?} -> {:?}", err.kind(), link_path, target_path, err.description());
    }
}
