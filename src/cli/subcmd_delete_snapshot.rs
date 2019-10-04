use clap;
use std;
use std::io::{stderr, Write};
use std::path::PathBuf;
use std::str::FromStr;

use crate::cli;
use crate::eerror::{EError, EResult};
use crate::snapshot;

pub fn sub_cmd<'a, 'b>() -> clap::App<'a, 'b> {
    clap::SubCommand::with_name("delete_snapshot")
        .visible_alias("del_ss")
        .about("Delete the specified snapshot(s)")
        .arg(
            clap::Arg::with_name("all_but_newest_n")
                .long("all_but_newest_n")
                .value_name("N")
                .takes_value(true)
                .help("delete all but the newest N snapshots")
                .required(true),
        )
        .arg(cli::arg_back_n().required(true))
        .group(
            clap::ArgGroup::with_name("which_ss")
                .args(&["all_but_newest_n", "back_n"])
                .required(true),
        )
        .arg(
            clap::Arg::with_name("remove_last_ok")
                .long("remove_last_ok")
                .takes_value(false)
                .help("authorise deletion of the last snapshot in the archive."),
        )
        .arg(
            cli::arg_archive_name()
                .required(true)
                .help("the name of the archive whose snapshot(s) are to be deleted"),
        )
        .arg(cli::arg_exigency_dir_path().help(
            "the name of the directory containing the snapshots to be deleted.
This option is intended for use in those cases where the configuration
data has been lost (possibly due to file system failure).  Individual
snapshot files contain sufficient data for orderly deletion without
the need for the configuration files provided their content repositories
are also intact.",
        ))
        .group(
            clap::ArgGroup::with_name("which")
                .args(&["archive_name", "exigency_dir_path"])
                .required(true),
        )
        .arg(cli::arg_verbose().help("report the number of snapshots deleted"))
}

pub fn run_cmd(arg_matches: &clap::ArgMatches) {
    let archive_or_dir_path = if let Some(archive_name) = arg_matches.value_of("archive_name") {
        snapshot::ArchiveOrDirPath::Archive(archive_name.to_string())
    } else if let Some(dir_path) = arg_matches.value_of("exigency_dir_path") {
        snapshot::ArchiveOrDirPath::DirPath(PathBuf::from(dir_path))
    } else {
        panic!("{:?}: line {:?}", file!(), line!())
    };
    let remove_last_ok = arg_matches.is_present("remove_last_ok");
    if let Some(n_as_str) = arg_matches.value_of("all_but_newest_n") {
        let n = match usize::from_str(n_as_str) {
            Ok(n) => n,
            Err(_) => {
                writeln!(stderr(), "Expected unsigned integer: found {}", n_as_str).unwrap();
                std::process::exit(1);
            }
        };
        match delete_all_but_newest(&archive_or_dir_path, n, remove_last_ok) {
            Ok(n) => {
                if arg_matches.is_present("verbose") {
                    println!("{} snapshots deleted", n)
                }
            }
            Err(err) => {
                writeln!(stderr(), "{:?}", err).unwrap();
                std::process::exit(1);
            }
        }
    } else if let Some(back_n_as_str) = arg_matches.value_of("back_n") {
        let n = match i64::from_str(back_n_as_str) {
            Ok(n) => n,
            Err(_) => {
                writeln!(stderr(), "Expected signed integer: found {}", back_n_as_str).unwrap();
                std::process::exit(1);
            }
        };
        match delete_ss_back_n(&archive_or_dir_path, n, remove_last_ok) {
            Ok(n) => {
                if arg_matches.is_present("verbose") {
                    println!("{} snapshots deleted", n)
                }
            }
            Err(err) => {
                writeln!(stderr(), "{:?}", err).unwrap();
                std::process::exit(1);
            }
        }
    } else {
        panic!("{:?}: line {:?}", file!(), line!())
    }
}

fn delete_all_but_newest(
    archive_or_dir_path: &snapshot::ArchiveOrDirPath,
    newest_count: usize,
    clear_fell: bool,
) -> EResult<(usize)> {
    let mut deleted_count: usize = 0;
    if !clear_fell && newest_count == 0 {
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

fn delete_ss_back_n(
    archive_or_dir_path: &snapshot::ArchiveOrDirPath,
    n: i64,
    clear_fell: bool,
) -> EResult<(usize)> {
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
    if !clear_fell && snapshot_paths.len() == 1 {
        return Err(EError::LastSnapshot(archive_or_dir_path.clone()));
    }
    snapshot::delete_snapshot_file(&snapshot_paths[index])?;
    Ok(1)
}
