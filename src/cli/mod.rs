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

use clap;

pub fn arg_archive_name<'a, 'b>() -> clap::Arg<'a, 'b> {
    clap::Arg::with_name("archive_name")
        .short("A").long("archive").value_name("name").takes_value(true)
}

pub fn arg_repo_name<'a, 'b>() -> clap::Arg<'a, 'b> {
    clap::Arg::with_name("repo_name")
        .short("R").long("repo").value_name("name").takes_value(true)
}

pub fn arg_file_path<'a, 'b>() -> clap::Arg<'a, 'b> {
    clap::Arg::with_name("file_path")
        .short("F").long("file").value_name("path").takes_value(true)
}

pub fn arg_location<'a, 'b>() -> clap::Arg<'a, 'b> {
    clap::Arg::with_name("location")
        .short("L").long("location").value_name("dir_path").takes_value(true)
}

pub fn arg_show_stats<'a, 'b>() -> clap::Arg<'a, 'b> {
    clap::Arg::with_name("show_stats")
        .long("stats").takes_value(false)
}

pub fn arg_verbose<'a, 'b>() -> clap::Arg<'a, 'b> {
    clap::Arg::with_name("verbose")
        .short("v").long("verbose").takes_value(false)
}

pub mod subcmd_back_up;
pub mod subcmd_delete_snapshot;
pub mod subcmd_delete_snapshot_file;
pub mod subcmd_list_archives;
pub mod subcmd_list_snapshots;
pub mod subcmd_new_archive;
pub mod subcmd_new_repo;
