//use std;
//use std::path::Path;
use clap;

//use cli;
use archive;

pub fn sub_cmd<'a, 'b>() -> clap::App<'a, 'b> {
    clap::SubCommand::with_name("list_archives").visible_alias("la")
        .about("List all defined snapshot archives")
}

pub fn run_cmd(_arg_matches: &clap::ArgMatches) {
    for archive_name in archive::get_archive_names() {
        println!("{}", archive_name);
    }
}
