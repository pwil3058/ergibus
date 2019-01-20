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

use ergibus::cli;

fn main() {
    let matches = clap::App::new("ergibus")
        .author(crate_authors!())
        .version(crate_version!())
        .subcommand(cli::subcmd_back_up::sub_cmd())
        .subcommand(cli::subcmd_delete_snapshot::sub_cmd())
        .subcommand(cli::subcmd_delete_snapshot_file::sub_cmd())
        .subcommand(cli::subcmd_extract::sub_cmd())
        .subcommand(cli::subcmd_list_archives::sub_cmd())
        .subcommand(cli::subcmd_list_snapshots::sub_cmd())
        .subcommand(cli::subcmd_new_archive::sub_cmd())
        .subcommand(cli::subcmd_new_repo::sub_cmd())
        .get_matches();

    match matches.subcommand() {
        ("back_up", Some(sub_matches)) => cli::subcmd_back_up::run_cmd(sub_matches),
        ("delete_snapshot", Some(sub_matches)) => cli::subcmd_delete_snapshot::run_cmd(sub_matches),
        ("delete_snapshot_file", Some(sub_matches)) => cli::subcmd_delete_snapshot_file::run_cmd(sub_matches),
        ("extract", Some(sub_matches)) => cli::subcmd_extract::run_cmd(sub_matches),
        ("list_archives", Some(sub_matches)) => cli::subcmd_list_archives::run_cmd(sub_matches),
        ("list_snapshots", Some(sub_matches)) => cli::subcmd_list_snapshots::run_cmd(sub_matches),
        ("new_archive", Some(sub_matches)) => cli::subcmd_new_archive::run_cmd(sub_matches),
        ("new_repo", Some(sub_matches)) => cli::subcmd_new_repo::run_cmd(sub_matches),
        _ => panic!("what happened")
    }
}
