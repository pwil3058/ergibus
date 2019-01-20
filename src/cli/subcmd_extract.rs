// Copyright 2019 Peter Williams <pwil3058@gmail.com>
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
use std::path::{PathBuf};

use cli;
use snapshot::ArchiveOrDirPath;

pub fn sub_cmd<'a, 'b>() -> clap::App<'a, 'b> {
    clap::SubCommand::with_name("extract")
        .about("Extract a copy of the nominated file/directory in the
nominated archive's most recent (or specified) snapshot
and place it in the current (or specified) directory.")
        .arg(cli::arg_back_n()
            .required(false)
        )
        .arg(cli::arg_archive_name()
            .required(true)
            .help("the name of the archive whose file or directory is to be extracted")
        )
        .arg(cli::arg_exigency_dir_path()
            .help(
"the name of the directory containing the snapshots whose file or
directory is to be extracted. This option is intended for use in those
cases where the configuration data has been lost (possibly due to file
system failure).  Individual snapshot files contain sufficient data for
extraction of files or directories without the need for the
configuration files provided their content repositories are also intact."
            )
        )
        .group(clap::ArgGroup::with_name("which")
            .args(&["archive_name", "exigency_dir_path"]).required(true)
        )
        .arg(cli::arg_file_path()
            .help("the path of the file to be copied.")
        )
        .arg(cli::arg_dir_path()
            .help("the path of the directory to be copied.")
        )
        .group(clap::ArgGroup::with_name("what")
            .args(&["file_path", "dir_path"]).required(false)
        )
        .arg(cli::arg_show_stats()
            .required(false).multiple(false)
            .help("show statistics for the extraction process")
        )
        .arg(cli::arg_overwrite()
            .required(false)
        )
        .arg(clap::Arg::with_name("with_name")
            .long("with_name").takes_value(true).value_name("name").required(false)
            .help("the name to be given to the copy of the file/directory.")
        )
        .arg(clap::Arg::with_name("into_dir")
            .long("into_dir").takes_value(true).value_name("path").required(false)
            .help("the path of the directory into which the file/directory is to be copied.")
        )
}

pub fn run_cmd(arg_matches: &clap::ArgMatches) {
    let _archive_or_dir_path = if let Some(archive_name) = arg_matches.value_of("archive_name") {
        ArchiveOrDirPath::Archive(archive_name.to_string())
    } else if let Some(dir_path) = arg_matches.value_of("exigency_dir_path") {
        ArchiveOrDirPath::DirPath(PathBuf::from(dir_path))
    } else {
        panic!("{:?}: line {:?}", file!(), line!())
    };
    if let Some(_file_path) = arg_matches.value_of("file_path") {

    } else if let Some(_dir_path) = arg_matches.value_of("dir_path") {

    } else {
        panic!("{:?}: line {:?}", file!(), line!())
    }
}
