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

use cli;
use snapshot;

pub fn sub_cmd<'a, 'b>() -> clap::App<'a, 'b> {
    clap::SubCommand::with_name("list_snapshots").visible_alias("lss")
        .about("List the snapshots for a nominated archive (or in a nominated directory)")
        .arg(cli::arg_archive_name()
            .required(true)
            .help("the name of the archive for whose snapshots are to be listed")
        )
        .arg(clap::Arg::with_name("exigency_dir_path")
            .short("X").long("exigency").value_name("dir_path")
            .required(true).takes_value(true)
            .long_help(
"the path of the directory containing the snapshots that are to be listed.
This option is intended for use in those cases where the configuration
data has been lost (possibly due to file system failure).  Individual
snapshot files contain sufficient data for file recovery/extraction
without the need for the configuration files provided their content
repositories are also intact."
            )
        )
        .group(clap::ArgGroup::with_name("which")
            .args(&["archive_name", "exigency_dir_path"]).required(true)
        )
}

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
