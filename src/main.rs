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

use std::path::PathBuf;

use ergibus::archive;
use ergibus::content;
use ergibus::snapshot;

fn backup_command(arg_matches: &clap::ArgMatches) {
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

fn delete_command(arg_matches: &clap::ArgMatches) {
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

fn new_archive_command(arg_matches: &clap::ArgMatches) {
    let archive_name = arg_matches.value_of("archive_name").ok_or(0).unwrap_or_else(
        |_| panic!("{:?}: line {:?}", file!(), line!())
    );
    let repo_name = arg_matches.value_of("repo_name").ok_or(0).unwrap_or_else(
        |_| panic!("{:?}: line {:?}", file!(), line!())
    );
    let location = arg_matches.value_of("location").ok_or(0).unwrap_or_else(
        |_| panic!("{:?}: line {:?}", file!(), line!())
    );
    let mut inclusions: Vec<String> = Vec::new();
    match arg_matches.values_of("inclusions") {
        Some(inclusion_values) => for inclusion in inclusion_values {
            inclusions.push(inclusion.to_string());
        },
        None => ()
    }
    let mut file_exclusions: Vec<String> = Vec::new();
    match arg_matches.values_of("file_exclusions") {
        Some(exclusion_values) => for exclusion in exclusion_values {
            file_exclusions.push(exclusion.to_string());
        },
        None => ()
    }
    let mut dir_exclusions: Vec<String> = Vec::new();
    match arg_matches.values_of("dir_exclusions") {
        Some(exclusion_values) => for exclusion in exclusion_values {
            dir_exclusions.push(exclusion.to_string());
        },
        None => ()
    }
    if let Err(err) = archive::create_new_archive(archive_name, repo_name, location, inclusions, dir_exclusions, file_exclusions) {
        println!("{:?}", err);
        std::process::exit(1);
    };
}

fn new_repo_command(arg_matches: &clap::ArgMatches) {
    println!("{:?}", arg_matches);
    let repo_name = arg_matches.value_of("repo_name").ok_or(0).unwrap_or_else(
        |_| panic!("{:?}: line {:?}", file!(), line!())
    );
    println!("Repository name: {:?}", repo_name);
    let location = arg_matches.value_of("location").ok_or(0).unwrap_or_else(
        |_| panic!("{:?}: line {:?}", file!(), line!())
    );
    println!("Location: {:?}", location);
    let algorithm = arg_matches.value_of("key_hash_algorithm").ok_or(0).unwrap_or_else(
        |_| panic!("{:?}: line {:?}", file!(), line!())
    );
    println!("Algorithm: {:?}", algorithm);
    if let Err(err) = content::create_new_repo(repo_name, location, algorithm) {
        println!("{:?}", err);
        std::process::exit(1);
    };
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
        (@subcommand new_archive =>
            (about: "Create a new content repository or a new snapshot archive")
            (visible_alias: "newa")
            (@arg archive_name:
                -A --archive <name>
                "the name of the new snapshot archive to be created"
            )
            (@arg repo_name:
                -R --repo <name>
                 "the name of the repository that the new archive should use to store file contents"
            )
            (@arg location:
                -L --location <dir_path> *
                "the directory path of the location where the archive should store its snapshots"
            )
            (@arg inclusions:
                -I --include <path> ...
                "the path of a file/directory that should be included in the archive's snapshots"
            )
            (@arg file_exclusions:
                -F --exclude_files_matching [glob] ...
                "exclude files matching this glob expression"
            )
            (@arg dir_exclusions:
                -D --exclude_dirs_matching [glob] ...
                "exclude directories matching this glob expression"
            )
        )
        (@subcommand new_repo =>
            (about: "Create a new content repository")
            (visible_alias: "newr")
            (@arg repo_name:
                -R --repo <name>
                "the name of the new content repository to be created"
            )
            (@arg location:
                -L --location <dir_path> *
                "the directory path of the location where the repo should store its data"
            )
            (@arg key_hash_algorithm:
                -K --key_hash_algorithm [algorithm]
                default_value[Sha256] //possible_values[Sha1 Sha256 Sha512]
                "the hash algorithm to use when generating repo content keys (Sha1, Sha256, Sha512)"
            )
        )
    ).get_matches();
    match matches.subcommand() {
        ("bu", Some(sub_matches)) => backup_command(sub_matches),
        ("del", Some(sub_matches)) => delete_command(sub_matches),
        ("new_archive", Some(sub_matches)) => new_archive_command(sub_matches),
        ("new_repo", Some(sub_matches)) => new_repo_command(sub_matches),
        _ => panic!("what happened")
    }
}
