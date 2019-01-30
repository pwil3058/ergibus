// Copyright 2019 Peter Williams <pwil3058@gmail.com>
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

use std::convert::From;
use std::ffi::CString;
use std::fs::{Metadata};
use std::io;
#[cfg(target_family = "unix")]
use std::os::unix::fs::MetadataExt;
#[cfg(target_family = "unix")]
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use libc;

pub trait AttributesIfce: From<Metadata> {
    fn size(&self) -> u64;
    fn set_file_attributes<W>(&self, file_path: &Path, op_errf: &mut Option<&mut W>) -> Result<(), io::Error>
        where W: std::io::Write;
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
#[cfg(target_family = "unix")]
pub struct Attributes {
    st_dev: u64,
    st_ino: u64,
    st_nlink: u64,
    st_mode: u32,
    st_uid: u32,
    st_gid: u32,
    st_size: u64,
    st_atime: i64,
    st_atime_nsec: i64,
    st_mtime: i64,
    st_mtime_nsec: i64,
    st_ctime: i64,
    st_ctime_nsec: i64,
}

#[cfg(target_family = "unix")]
impl Attributes {
    pub fn chmod_file(&self, file_path: &Path) -> Result<(), io::Error> {
        let c_file_path = CString::new(file_path.as_os_str().as_bytes()).unwrap();
        let failed: bool;
        unsafe {
            failed = libc::chmod(c_file_path.into_raw(), self.st_mode) != 0;
        }
        if failed {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub fn chown_file(&self, file_path: &Path) -> Result<(), io::Error> {
        let c_file_path = CString::new(file_path.as_os_str().as_bytes()).unwrap();
        let failed: bool;
        unsafe {
            failed = libc::chown(c_file_path.into_raw(), self.st_uid, self.st_gid) != 0;
        }
        if failed {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub fn utime_file(&self, file_path: &Path) -> Result<(), io::Error> {
        let c_file_path = CString::new(file_path.as_os_str().as_bytes()).unwrap();
        let time_values = libc::utimbuf{actime: self.st_atime, modtime: self.st_mtime};
        let failed: bool;
        unsafe {
            failed = libc::utime(c_file_path.into_raw(), &time_values) != 0;
        }
        if failed {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

#[cfg(target_family = "unix")]
impl From<Metadata> for Attributes {
    fn from(metadata: Metadata) -> Attributes {
        Attributes{
            st_dev: metadata.dev(),
            st_ino: metadata.ino(),
            st_nlink: metadata.nlink(),
            st_mode: metadata.mode(),
            st_uid: metadata.uid(),
            st_gid: metadata.gid(),
            st_size: metadata.size(),
            st_atime: metadata.atime(),
            st_atime_nsec: metadata.atime_nsec(),
            st_mtime: metadata.mtime(),
            st_mtime_nsec: metadata.mtime_nsec(),
            st_ctime: metadata.ctime(),
            st_ctime_nsec: metadata.ctime_nsec(),
        }
    }
}

#[cfg(target_family = "unix")]
impl AttributesIfce for Attributes {
    fn size(&self) -> u64 {
        self.st_size
    }

    fn set_file_attributes<W>(&self, file_path: &Path, op_errf: &mut Option<&mut W>) -> Result<(), io::Error>
        where W: std::io::Write
    {
        if let Err(err) = self.chmod_file(file_path) {
            match op_errf {
                Some(ref mut errf) => writeln!(errf, "{:?}: {}", file_path, err).unwrap(),
                None => return Err(err)
            };
        }
        if let Err(err) = self.utime_file(file_path) {
            match op_errf {
                Some(ref mut errf) => writeln!(errf, "{:?}: {}", file_path, err).unwrap(),
                None => return Err(err)
            };
        }
        if let Err(err) = self.chown_file(file_path) {
            match op_errf {
                Some(ref mut errf) => writeln!(errf, "{:?}: {}", file_path, err).unwrap(),
                None => return Err(err)
            };
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    //use super::*;

    #[test]
    fn it_works() {

    }
}
