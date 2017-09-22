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
use std::path::Path;
use clap;
use snapshot;

pub fn run_cmd(arg_matches: &clap::ArgMatches) {
    if arg_matches.is_present("archive_name") {
        let archive_name = arg_matches.value_of("archive_name").ok_or(0).unwrap_or_else(
            |_| panic!("{:?}: line {:?}", file!(), line!())
        );
        match snapshot::get_snapshot_names_for_archive(&archive_name) {
            Ok(snapshot_names) => for name in snapshot_names {
                println!("{:?}", name);
            },
            Err(err) => {
                println!("{:?}", err);
                std::process::exit(1);
            }
        }
    } else {
        let exigency_dir_path = arg_matches.value_of("exigency_dir_path").ok_or(0).unwrap_or_else(
            |_| panic!("{:?}: line {:?}", file!(), line!())
        );
        match snapshot::get_snapshot_names_in_dir(Path::new(&exigency_dir_path)) {
            Ok(snapshot_names) => for name in snapshot_names {
                println!("{:?}", name);
            },
            Err(err) => {
                println!("{:?}", err);
                std::process::exit(1);
            }
        }
    }
}
