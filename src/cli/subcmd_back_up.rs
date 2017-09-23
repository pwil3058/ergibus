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
    clap::SubCommand::with_name("back_up").visible_alias("bu")
        .about("Generate a backup snapshot for the specified archive(s)")
        .arg(cli::arg_archive_name()
            .required(true).multiple(true)
            .help("the name of an archive to generate backup snapshot for")
        )
}

pub fn run_cmd(arg_matches: &clap::ArgMatches) {
    let mut had_errors = false;
    let archives = arg_matches.values_of("archive").ok_or(0).unwrap_or_else(
        |_| panic!("{:?}: line {:?}", file!(), line!())
    );
    for archive in archives {
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
