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
use content;

pub fn sub_cmd<'a, 'b>() -> clap::App<'a, 'b> {
    clap::SubCommand::with_name("new_repo").visible_alias("newr")
        .about("Create a new content repository")
        .arg(cli::arg_file_path()
            .required(true)
            .help("the name of the new content repository to be created")
        )
        .arg(cli::arg_location()
            .required(true)
            .long_help(
"the directory path of the location where the repository should store its data"
            )
        )
        .arg(clap::Arg::with_name("token_hash_algorithm")
            .short("T").long("token_hash_algorithm").value_name("algorithm")
            .required(false).takes_value(true)
            .possible_values(&["Sha1", "Sha256", "Sha512"])
            .default_value("Sha256")
            .help("the hash algorithm to use when generating repository's file content tokens")
        )
}

pub fn run_cmd(arg_matches: &clap::ArgMatches) {
    let repo_name = arg_matches.value_of("repo_name").ok_or(0).unwrap_or_else(
        |_| panic!("{:?}: line {:?}", file!(), line!())
    );
    let location = arg_matches.value_of("location").ok_or(0).unwrap_or_else(
        |_| panic!("{:?}: line {:?}", file!(), line!())
    );
    let algorithm = arg_matches.value_of("token_hash_algorithm").ok_or(0).unwrap_or_else(
        |_| panic!("{:?}: line {:?}", file!(), line!())
    );
    if let Err(err) = content::create_new_repo(repo_name, location, algorithm) {
        println!("{:?}", err);
        std::process::exit(1);
    };
}