use std::error::Error;
use std::io::{self, ErrorKind};
use std::path::{Path};

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
