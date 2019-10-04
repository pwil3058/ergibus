use clap;
use std;
use std::path::PathBuf;

use crate::cli;
use crate::snapshot;

pub fn sub_cmd<'a, 'b>() -> clap::App<'a, 'b> {
    clap::SubCommand::with_name("delete_snapshot_file")
        .visible_alias("del_ss_file")
        .about("Delete the specified snapshot file(s)")
        .arg(
            cli::arg_file_path()
                .required(true)
                .multiple(true)
                .help("path of snapshot file to be deleted"),
        )
}

pub fn run_cmd(arg_matches: &clap::ArgMatches) {
    let mut had_errors = false;
    let files = arg_matches
        .values_of("file")
        .ok_or(0)
        .unwrap_or_else(|_| panic!("{:?}: line {:?}", file!(), line!()));
    for file in files {
        let path = PathBuf::from(file);
        match snapshot::delete_snapshot_file(&path) {
            Ok(()) => {}
            Err(err) => {
                println!("{:?}", err);
                had_errors = true;
            }
        }
    }
    if had_errors {
        std::process::exit(1);
    }
}
