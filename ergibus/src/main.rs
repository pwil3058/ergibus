// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

mod archive_sub_cmds;
mod repository_sub_cmds;
mod snapshot_sub_cmds;

use structopt::StructOpt;

use crate::archive_sub_cmds::ManageArchives;
use crate::repository_sub_cmds::ManageRepositories;
use crate::snapshot_sub_cmds::{BackUp, SnapshotContents, SnapshotManager};

#[derive(Debug, StructOpt)]
/// Experimental Rust Git Inspired Back Up System
enum Ergibus {
    /// Manage archives
    #[structopt(alias = "ar")]
    Archive(ManageArchives),
    /// Manage repositories
    #[structopt(alias = "re")]
    Repo(ManageRepositories),
    /// Manage archive snapshots
    #[structopt(alias = "ms")]
    ManageSnapshots(SnapshotManager),
    /// Manage snapshot contents
    #[structopt(alias = "sc")]
    SnapshotContents(SnapshotContents),
    /// Take backup snapshots
    #[structopt(alias = "bu")]
    BackUp(BackUp),
}

fn main() {
    let ergibus = Ergibus::from_args();

    if let Err(err) = match ergibus {
        Ergibus::Archive(sub_cmd) => sub_cmd.exec(),
        Ergibus::Repo(sub_cmd) => sub_cmd.exec(),
        Ergibus::ManageSnapshots(sub_cmd) => sub_cmd.exec(),
        Ergibus::SnapshotContents(sub_cmd) => sub_cmd.exec(),
        Ergibus::BackUp(sub_cmd) => sub_cmd.exec(),
    } {
        println!("failed: {:?}", err);
        std::process::exit(1);
    }
}
