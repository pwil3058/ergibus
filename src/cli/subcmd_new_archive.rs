use clap;
use std;

use crate::archive;
use crate::cli;

pub fn sub_cmd<'a, 'b>() -> clap::App<'a, 'b> {
    clap::SubCommand::with_name("new_archive").visible_alias("newa")
        .about("Create a new snapshot archive")
        .arg(cli::arg_archive_name()
            .required(true)
            .help("the name of the new snapshot archive to be created")
        )
        .arg(cli::arg_repo_name()
            .required(true)
            .help("the name of the repository that the new archive should use to store file contents")
        )
        .arg(cli::arg_location()
            .required(true)
            .long_help(
"the directory path of the location where the archive should store its snapshots"
            )
        )
        .arg(clap::Arg::with_name("inclusions")
            .short("I").long("include").value_name("name")
            .required(true).takes_value(true).multiple(true)
            .help("the path of a file/directory that should be included in the archive's snapshots")
        )
        .arg(clap::Arg::with_name("dir_exclusions")
            .short("D").long("exclude_dirs").value_name("glob")
            .required(false).takes_value(true).multiple(true)
            .help("exclude directories matching this glob expression from patches")
        )
        .arg(clap::Arg::with_name("file_exclusions")
            .short("F").long("exclude_files").value_name("glob")
            .required(false).takes_value(true).multiple(true)
            .help("exclude files matching this glob expression from patches")
        )
}

pub fn run_cmd(arg_matches: &clap::ArgMatches<'_>) {
    let archive_name = arg_matches
        .value_of("archive_name")
        .ok_or(0)
        .unwrap_or_else(|_| panic!("{:?}: line {:?}", file!(), line!()));
    let repo_name = arg_matches
        .value_of("repo_name")
        .ok_or(0)
        .unwrap_or_else(|_| panic!("{:?}: line {:?}", file!(), line!()));
    let location = arg_matches
        .value_of("location")
        .ok_or(0)
        .unwrap_or_else(|_| panic!("{:?}: line {:?}", file!(), line!()));
    let mut inclusions: Vec<String> = Vec::new();
    match arg_matches.values_of("inclusions") {
        Some(inclusion_values) => {
            for inclusion in inclusion_values {
                inclusions.push(inclusion.to_string());
            }
        }
        None => (),
    }
    let mut file_exclusions: Vec<String> = Vec::new();
    match arg_matches.values_of("file_exclusions") {
        Some(exclusion_values) => {
            for exclusion in exclusion_values {
                file_exclusions.push(exclusion.to_string());
            }
        }
        None => (),
    }
    let mut dir_exclusions: Vec<String> = Vec::new();
    match arg_matches.values_of("dir_exclusions") {
        Some(exclusion_values) => {
            for exclusion in exclusion_values {
                dir_exclusions.push(exclusion.to_string());
            }
        }
        None => (),
    }
    if let Err(err) = archive::create_new_archive(
        archive_name,
        repo_name,
        location,
        inclusions,
        dir_exclusions,
        file_exclusions,
    ) {
        println!("{:?}", err);
        std::process::exit(1);
    };
}
