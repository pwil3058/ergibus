// Copyright 2019 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>
use std::path::PathBuf;
use structopt::StructOpt;

use crate::eerror::{EError, EResult};
use crate::snapshot::{self, ArchiveOrDirPath};

#[derive(Debug, StructOpt)]
pub struct Snapshot {
    /// the name of the snapshot archive that contains the snapshot(s) to be acted on.
    #[structopt(short = "A", long = "archive", group = "which")]
    archive_name: Option<String>,
    /// the name of the directory containing the snapshot(s) to be acted on.
    /// This option is intended for use in those
    /// cases where the configuration data has been lost (possibly due to file
    /// system failure).  Individual snapshot files contain sufficient data for
    /// extraction of files or directories without the need for the
    /// configuration files provided their content repositories are also intact.
    #[structopt(short = "X", long = "exigency", group = "which", parse(from_os_str))]
    exigency_dir_path: Option<PathBuf>,
    #[structopt(subcommand)]
    sub_cmd: SubCmd,
}

#[derive(Debug, StructOpt)]
pub enum SubCmd {
    /// List the snapshots for a nominated archive (or in a nominated directory).
    List,
    /// Delete the specified snapshot(s).
    Delete(Delete),
}

#[derive(Debug, StructOpt)]
#[structopt(group = clap::ArgGroup::with_name("which_ss").required(true))]
pub struct Delete {
    /// all but newest `N` snapshots.
    #[structopt(value_name = "N", group = "which_ss")]
    all_but_newest_n: Option<usize>,
    /// delete the snapshot "N" places before the most recent. Use -1 to select oldest.
    #[structopt(value_name = "N", group = "which_ss")]
    back_n: Option<i64>,
    /// authorise deletion of the last remaining snapshot in the archive.
    clear_fell: bool,
    /// authorise deletion of the last remaining snapshot in the archive.
    verbose: bool,
}

impl Delete {
    pub fn exec(&self, archive_or_dir_path: &ArchiveOrDirPath) {
        if let Some(count) = self.all_but_newest_n {
            match self.delete_all_but_newest(archive_or_dir_path, count) {
                Ok(number) => {
                    if self.verbose {
                        println!("{} snapshots deleted.", number)
                    }
                }
                Err(err) => {
                    // TODO: send error messages to stderr
                    println!("{:?}", err);
                    std::process::exit(1);
                }
            }
        } else if let Some(back_n) = self.back_n {
            match self.delete_ss_back_n(archive_or_dir_path, back_n) {
                Ok(number) => {
                    if self.verbose {
                        println!("{} snapshots deleted.", number)
                    }
                }
                Err(err) => {
                    // TODO: send error messages to stderr
                    println!("{:?}", err);
                    std::process::exit(1);
                }
            }
        } else {
            panic!("clap shouldn't le us get here")
        }
    }

    fn delete_all_but_newest(
        &self,
        archive_or_dir_path: &ArchiveOrDirPath,
        newest_count: usize,
    ) -> EResult<(usize)> {
        let mut deleted_count: usize = 0;
        if !self.clear_fell && newest_count == 0 {
            return Err(EError::LastSnapshot(archive_or_dir_path.clone()));
        }
        let snapshot_paths = archive_or_dir_path.get_snapshot_paths(false)?;
        if snapshot_paths.len() == 0 {
            return Err(EError::ArchiveEmpty(archive_or_dir_path.clone()));
        }
        if snapshot_paths.len() <= newest_count {
            return Ok(0);
        }
        let last_index = snapshot_paths.len() - newest_count;
        for snapshot_path in snapshot_paths[0..last_index].iter() {
            snapshot::delete_snapshot_file(snapshot_path)?;
            deleted_count += 1;
        }
        Ok(deleted_count)
    }

    fn delete_ss_back_n(&self, archive_or_dir_path: &ArchiveOrDirPath, n: i64) -> EResult<(usize)> {
        let snapshot_paths = archive_or_dir_path.get_snapshot_paths(true)?;
        if snapshot_paths.len() == 0 {
            return Err(EError::ArchiveEmpty(archive_or_dir_path.clone()));
        };
        let index: usize = if n < 0 {
            (snapshot_paths.len() as i64 + n) as usize
        } else {
            n as usize
        };
        if snapshot_paths.len() <= index {
            return Ok(0);
        }
        if !self.clear_fell && snapshot_paths.len() == 1 {
            return Err(EError::LastSnapshot(archive_or_dir_path.clone()));
        }
        snapshot::delete_snapshot_file(&snapshot_paths[index])?;
        Ok(1)
    }
}

impl Snapshot {
    pub fn exec(&self) {
        let archive_or_dir_path = if let Some(archive_name) = &self.archive_name {
            ArchiveOrDirPath::Archive(archive_name.clone())
        } else if let Some(dir_path) = &self.exigency_dir_path {
            ArchiveOrDirPath::DirPath(dir_path.to_path_buf())
        } else {
            println!("either --archive or --exigency must be present");
            std::process::exit(1);
        };
        match self.sub_cmd {
            SubCmd::List => match archive_or_dir_path.get_snapshot_names(false) {
                Ok(snapshot_names) => {
                    for name in snapshot_names {
                        println!("{:?}", name);
                    }
                }
                Err(err) => {
                    println!("{:?}", err);
                    std::process::exit(1);
                }
            },
            SubCmd::Delete(ref delete) => delete.exec(&archive_or_dir_path),
        }
    }
}
