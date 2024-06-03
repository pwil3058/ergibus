// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

mod archive_sub_cmds;
mod snapshot_sub_cmds;

use log::*;
use stderrlog;
use structopt::StructOpt;

use crate::archive_sub_cmds::ManageArchives;
use crate::snapshot_sub_cmds::{BackUp, SnapshotContents, SnapshotManager};

/// A StructOpt example
#[derive(StructOpt, Debug)]
#[structopt()]
struct Ergibus {
    /// Silence all output
    #[structopt(short = "q", long = "quiet")]
    quiet: bool,
    /// Verbose mode (-v, -vv, -vvv, etc)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: usize,
    /// Timestamp (sec, ms, ns, none)
    #[structopt(short = "t", long = "timestamp")]
    ts: Option<stderrlog::Timestamp>,
    /// Sub commands
    #[structopt(subcommand)]
    sub_cmd: SubCommands,
}

#[derive(Debug, StructOpt)]
/// Experimental Rust Git Inspired Back Up System
enum SubCommands {
    /// Manage archives
    #[structopt(alias = "ar")]
    Archive(ManageArchives),
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

    stderrlog::new()
        //.module(module_path!())
        .quiet(ergibus.quiet)
        .verbosity(ergibus.verbose)
        .timestamp(ergibus.ts.unwrap_or(stderrlog::Timestamp::Off))
        .init()
        .unwrap();

    if let Err(err) = match ergibus.sub_cmd {
        SubCommands::Archive(sub_cmd) => sub_cmd.exec(),
        SubCommands::ManageSnapshots(sub_cmd) => sub_cmd.exec(),
        SubCommands::SnapshotContents(sub_cmd) => sub_cmd.exec(),
        SubCommands::BackUp(sub_cmd) => sub_cmd.exec(),
    } {
        error!("{:?}", err);
        std::process::exit(1);
    }
}
