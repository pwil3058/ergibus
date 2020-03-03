use clap;

pub fn arg_archive_name<'a, 'b>() -> clap::Arg<'a, 'b> {
    clap::Arg::with_name("archive_name")
        .short("A")
        .long("archive")
        .value_name("name")
        .takes_value(true)
}

pub fn arg_repo_name<'a, 'b>() -> clap::Arg<'a, 'b> {
    clap::Arg::with_name("repo_name")
        .short("R")
        .long("repo")
        .value_name("name")
        .takes_value(true)
}

pub fn arg_dir_path<'a, 'b>() -> clap::Arg<'a, 'b> {
    clap::Arg::with_name("dir_path")
        .short("D")
        .long("dir")
        .value_name("path")
        .takes_value(true)
}

pub fn arg_file_path<'a, 'b>() -> clap::Arg<'a, 'b> {
    clap::Arg::with_name("file_path")
        .short("F")
        .long("file")
        .value_name("path")
        .takes_value(true)
}

pub fn arg_location<'a, 'b>() -> clap::Arg<'a, 'b> {
    clap::Arg::with_name("location")
        .short("L")
        .long("location")
        .value_name("dir_path")
        .takes_value(true)
}

pub fn arg_overwrite<'a, 'b>() -> clap::Arg<'a, 'b> {
    clap::Arg::with_name("overwrite")
        .long("overwrite")
        .takes_value(false)
        .help("overwrite a file/directory if it already exists instead of moving it aside.")
}

pub fn arg_show_stats<'a, 'b>() -> clap::Arg<'a, 'b> {
    clap::Arg::with_name("show_stats")
        .long("stats")
        .takes_value(false)
}

pub fn arg_verbose<'a, 'b>() -> clap::Arg<'a, 'b> {
    clap::Arg::with_name("verbose")
        .short("v")
        .long("verbose")
        .takes_value(false)
}

pub fn arg_exigency_dir_path<'a, 'b>() -> clap::Arg<'a, 'b> {
    clap::Arg::with_name("exigency_dir_path")
        .short("X")
        .long("exigency")
        .value_name("dir_path")
        .required(true)
        .takes_value(true)
}

pub fn arg_back_n<'a, 'b>() -> clap::Arg<'a, 'b> {
    clap::Arg::with_name("back_n")
        .long("back")
        .value_name("N")
        .takes_value(true)
        .help("select the snapshot \"N\" places before the most recent. Use -1 to select oldest.")
}
