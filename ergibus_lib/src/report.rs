use std::io::ErrorKind;
use std::path::Path;

use crate::{EResult, Error};
use log;

pub fn ignore_report_or_fail<P: AsRef<Path>>(err: Error, path: P) -> EResult<()> {
    match &err {
        Error::FSOBrokenSymLink(link_path, target_path) => {
            log::warn!(
                "{:?} -> {:?}: broken symbolic link ignored",
                link_path,
                target_path
            );
            Ok(())
        }
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
