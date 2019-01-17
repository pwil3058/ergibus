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

use std;
use std::path::PathBuf;
use clap;

use cli;
use snapshot;

pub fn sub_cmd<'a, 'b>() -> clap::App<'a, 'b> {
    clap::SubCommand::with_name("delete_snapshot_file").visible_alias("del_ss_file")
        .about("Delete the specified snapshot file(s)")
        .arg(cli::arg_file_path()
            .required(true).multiple(true)
            .help("path of snapshot file to be deleted")
        )
}

pub fn run_cmd(arg_matches: &clap::ArgMatches) {
    let mut had_errors = false;
    let files = arg_matches.values_of("file").ok_or(0).unwrap_or_else(
        |_| panic!("{:?}: line {:?}", file!(), line!())
    );
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
