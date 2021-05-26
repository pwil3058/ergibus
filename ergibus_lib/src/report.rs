use std::io::{self, ErrorKind};
use std::path::Path;

use crate::{EResult, Error};
use log;

pub fn ignore_report_or_fail<P: AsRef<Path>>(err: Error, path: P) -> EResult<()> {
    match &err {
        Error::IOError(io_err) => {
            match io_err.kind() {
                // we assume that "not found" is due to a race condition
                ErrorKind::NotFound => {
                    log::trace!("{:?}: not found", path.as_ref());
                    Ok(())
                }
                // benign so just report it
                ErrorKind::PermissionDenied => {
                    log::warn!("{:?}: permission denied", path.as_ref());
                    Ok(())
                }
                // programming error that needs to be fixed
                _ => Err(err),
            }
        }
        _ => Err(err),
    }
}

pub fn ignore_report_or_crash(err: &io::Error, path: &Path) {
    match err.kind() {
        // we assume that "not found" is due to a race condition
        ErrorKind::NotFound => log::trace!("{:?}: not found", path),
        // benign so just report it
        ErrorKind::PermissionDenied => log::warn!("{:?}: permission denied", path),
        // programming error that needs to be fixed
        _ => {
            log::error!("{:?}: {:?}: {:?}", err.kind(), err.to_string(), path);
            panic!("{:?}: {:?}: {:?}", err.kind(), err.to_string(), path);
        }
    }
}

pub fn report_broken_link_or_crash(err: &io::Error, link_path: &Path, target_path: &Path) {
    match err.kind() {
        ErrorKind::NotFound => log::warn!(
            "{:?} -> {:?}: broken symbolic link ignored",
            link_path,
            target_path
        ),
        _ => {
            log::error!(
                "{:?}: {:?}: {:?} -> {:?}",
                err.kind(),
                link_path,
                target_path,
                err.to_string()
            );
            panic!(
                "{:?}: {:?}: {:?} -> {:?}",
                err.kind(),
                link_path,
                target_path,
                err.to_string()
            );
        }
    }
}
