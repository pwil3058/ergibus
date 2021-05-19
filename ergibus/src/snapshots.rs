// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::convert::TryFrom;
use std::path::PathBuf;

use structopt::{clap::ArgGroup, StructOpt};

use ergibus_lib::{archive::SnapshotDir, EResult};

#[derive(Debug, StructOpt)]
#[structopt(group = ArgGroup::with_name("which").required(true))]
pub struct Snapshots {
    /// the name of the snapshot archive that contains the snapshot(s) to be acted on.
    #[structopt(short, long = "archive", group = "which")]
    archive_name: Option<String>,
    /// the name of the directory containing the snapshot(s) to be acted on.
    ///
    /// This option is intended for use in those
    /// cases where the configuration data has been lost (possibly due to file
    /// system failure).  Individual snapshot files contain sufficient data for
    /// extraction of files or directories without the need for the
    /// configuration files provided their content repositories are also intact.
    #[structopt(short = "x", long = "exigency", group = "which", parse(from_os_str))]
    exigency_dir_path: Option<PathBuf>,
    #[structopt(subcommand)]
    sub_cmd: SubCmd,
}

#[derive(Debug, StructOpt)]
pub enum SubCmd {
    /// List the snapshots for a nominated archive (or in a nominated directory).
    List,
    // /// Delete the specified snapshot(s).
    // Delete(Delete),
}

impl Snapshots {
    pub fn exec(&self) -> EResult<()> {
        let snapshot_dir = if let Some(archive_name) = &self.archive_name {
            SnapshotDir::try_from(archive_name.as_str())?
        } else if let Some(dir_path) = &self.exigency_dir_path {
            SnapshotDir::try_from(dir_path.as_path())?
        } else {
            panic!("either --archive or --exigency must be present");
        };
        match self.sub_cmd {
            SubCmd::List => {
                for name in snapshot_dir.get_snapshot_names(false)?.iter() {
                    println!("{:?}", name);
                }
            } //     SubCmd::Delete(ref delete) => delete.exec(&snapshot_dir),
        }
        Ok(())
    }
}
