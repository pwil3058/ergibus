// Copyright 2017 Peter Williams <pwil3058@gmail.com>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[macro_use]
extern crate clap;

extern crate ergibus;

use std::io::{stdout, stderr};
use std::path::PathBuf;
use std::str::FromStr;

use ergibus::snapshot;

fn backup_command(arg_matches: &clap::ArgMatches) {
    let mut had_errors = false;
    // safe to unwrap here as "archive" is a required option
    for archive in arg_matches.values_of("archive").unwrap() {
        match snapshot::generate_snapshot(&archive) {
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

fn delete_command(arg_matches: &clap::ArgMatches) {
    let mut had_errors = false;
    // safe to unwrap here as "file" is a required option
    for file in arg_matches.values_of("file").unwrap() {
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

fn main() {
    let matches = clap_app!(ergibus =>
        (author: "Peter Williams <pwil3058@gmail.com>")
        (about: "manage file backups")
        (@subcommand bu =>
            (about: "Generate a backup snapshot for the specified archive(s)")
            (@arg archive:
                -A --archive ...
                +required +takes_value
                "the name of the archive to generate backup snapshot for"
            )
        )
        (@subcommand del =>
            (about: "Delete the specified snapshot file(s)")
            (@arg file:
                -F --file ...
                +required +takes_value
                "path of snapshot file to be deleted"
            )
        )
    ).get_matches();
    match matches.subcommand() {
        ("bu", Some(sub_matches)) => backup_command(sub_matches),
        ("del", Some(sub_matches)) => delete_command(sub_matches),
        _ => panic!("what happened")
    }
}
