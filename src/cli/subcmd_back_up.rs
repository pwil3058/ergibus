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
//use std::path::Path;
use clap;

use cli;
use snapshot;

pub fn sub_cmd<'a, 'b>() -> clap::App<'a, 'b> {
    clap::SubCommand::with_name("back_up").visible_alias("bu")
        .about("Generate a backup snapshot for the specified archive(s)")
        .arg(cli::arg_show_stats()
            .required(false).multiple(false)
            .help("show statistics for generated snapshots")
        )
        .arg(cli::arg_archive_name()
            .required(true).multiple(true)
            .help("the name of an archive to generate backup snapshot for")
        )
}

pub fn run_cmd(arg_matches: &clap::ArgMatches) {
    let mut had_errors = false;
    let archives = arg_matches.values_of("archive_name").unwrap_or_else(
        || panic!("{:?}: line {:?} {:?}", file!(), line!(), arg_matches)
    );
    let show_stats = arg_matches.is_present("show_stats");
    if show_stats {
        println!("{:>12} | {:>12} | {:>12} | {:>12} | {:>8} | {:>8} | {:>14} | {}",
            "#Files", "#Bytes", "#Stored", "#Change",
            "#Dir SL", "#File SL", "Time taken", "Archive Name"
        );
    }
    for archive in archives {
        match snapshot::generate_snapshot(&archive) {
            Ok(stats) => if show_stats {
                let time_taken = format!("{:?}", stats.0);
                println!("{:>12} | {:>12} | {:>12} | {:>12} | {:>8} | {:>8} | {:>14} | {}",
                    stats.1.file_count,
                    stats.1.byte_count,
                    stats.1.stored_byte_count,
                    stats.3,
                    stats.2.dir_sym_link_count,
                    stats.2.file_sym_link_count,
                    time_taken,
                    archive,
                );
            },
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
