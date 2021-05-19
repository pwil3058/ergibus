use clap;
use std;
use std::io::{stderr, Write};
use std::path::PathBuf;
use std::str::FromStr;

use crate::cli;
use ergibus_lib::archive;
use std::convert::TryFrom;

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

pub fn run_cmd(arg_matches: &clap::ArgMatches<'_>) {
    let snapshot_dir = if let Some(archive_name) = arg_matches.value_of("archive_name") {
        archive::SnapshotDir::try_from(archive_name).expect("miraculously no bad names given")
    } else if let Some(dir_path) = arg_matches.value_of("exigency_dir_path") {
        let path = PathBuf::from(dir_path);
        archive::SnapshotDir::try_from(path.as_path()).expect("miraculously no bad names given")
    } else {
        panic!("either --archive or --exigency must be present")
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
        match snapshot_dir.delete_all_but_newest(n, remove_last_ok) {
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
        match snapshot_dir.delete_ss_back_n(n, remove_last_ok) {
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
