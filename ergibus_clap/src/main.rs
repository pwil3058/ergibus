#[macro_use]
extern crate clap;

use structopt::StructOpt;

pub mod cli;
pub mod subcmd_archive;
pub mod subcmd_back_up;
pub mod subcmd_delete_snapshot;
pub mod subcmd_delete_snapshot_file;
pub mod subcmd_extract;
pub mod subcmd_list_archives;
pub mod subcmd_list_snapshots;
pub mod subcmd_new_archive;
pub mod subcmd_new_repo;
pub mod subcmd_repo;
pub mod subcmd_snapshot;

use crate::subcmd_archive::Archive;
use crate::subcmd_back_up::BackUp;
use crate::subcmd_extract::Extract;
use crate::subcmd_repo::Repository;
use crate::subcmd_snapshot::Snapshot;

#[derive(StructOpt)]
#[structopt(about = "Experimental Rust Git Inspired Back Up System", author = crate_authors!())]
enum Ergibus {
    /// Generate a backup snapshot for the specified archive(s).
    #[structopt(alias = "bu")]
    BackUp(BackUp),
    /// Manage archives.
    #[structopt(alias = "ar")]
    Archive(Archive),
    /// Manage content repositories.
    #[structopt(alias = "repo")]
    Repository(Repository),
    /// Extract a file or directory from a snapshot.
    Extract(Extract),
    /// Manage snapshot files.
    #[structopt(alias = "ss")]
    Snapshot(Snapshot),
}

fn _alternate_main() {
    let ergibus = Ergibus::from_args();
    match ergibus {
        Ergibus::BackUp(back_up) => back_up.exec(),
        Ergibus::Archive(archive) => archive.exec(),
        Ergibus::Repository(repository) => repository.exec(),
        Ergibus::Extract(extract) => extract.exec(),
        Ergibus::Snapshot(snapshot) => snapshot.exec(),
    }
}

fn main() {
    let matches = clap::App::new("ergibus_clap")
        .author(crate_authors!())
        .version(crate_version!())
        .subcommand(subcmd_back_up::sub_cmd())
        .subcommand(subcmd_delete_snapshot::sub_cmd())
        .subcommand(subcmd_delete_snapshot_file::sub_cmd())
        .subcommand(subcmd_extract::sub_cmd())
        .subcommand(subcmd_list_archives::sub_cmd())
        .subcommand(subcmd_list_snapshots::sub_cmd())
        .subcommand(subcmd_new_archive::sub_cmd())
        .subcommand(subcmd_new_repo::sub_cmd())
        .get_matches();

    match matches.subcommand() {
        ("back_up", Some(sub_matches)) => subcmd_back_up::run_cmd(sub_matches),
        ("delete_snapshot", Some(sub_matches)) => subcmd_delete_snapshot::run_cmd(sub_matches),
        ("delete_snapshot_file", Some(sub_matches)) => {
            subcmd_delete_snapshot_file::run_cmd(sub_matches)
        }
        ("extract", Some(sub_matches)) => subcmd_extract::run_cmd(sub_matches),
        ("list_archives", Some(sub_matches)) => subcmd_list_archives::run_cmd(sub_matches),
        ("list_snapshots", Some(sub_matches)) => subcmd_list_snapshots::run_cmd(sub_matches),
        ("new_archive", Some(sub_matches)) => subcmd_new_archive::run_cmd(sub_matches),
        ("new_repo", Some(sub_matches)) => subcmd_new_repo::run_cmd(sub_matches),
        _ => panic!("what happened"),
    }
}
