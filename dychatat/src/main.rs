// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

mod sub_cmds;

use log::*;
use stderrlog;
use structopt::StructOpt;

use sub_cmds::ManageRepositories;

/// A StructOpt example
#[derive(StructOpt, Debug)]
#[structopt()]
struct Dychatat {
    /// Silence all output
    #[structopt(short = "q", long = "quiet")]
    quiet: bool,
    /// Verbose mode (-v, -vv, -vvv, etc)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: usize,
    /// Timestamp (sec, ms, ns, none)
    #[structopt(short = "t", long = "timestamp")]
    ts: Option<stderrlog::Timestamp>,
    /// Sub commands
    #[structopt(subcommand)]
    sub_cmd: ManageRepositories,
}

fn main() {
    let dychatat = Dychatat::from_args();

    stderrlog::new()
        //.module(module_path!())
        .quiet(dychatat.quiet)
        .verbosity(dychatat.verbose)
        .timestamp(dychatat.ts.unwrap_or(stderrlog::Timestamp::Off))
        .init()
        .unwrap();

    if let Err(err) = match dychatat.sub_cmd {
        ManageRepositories::Delete(sub_cmd) => sub_cmd.exec(),
        ManageRepositories::List(sub_cmd) => sub_cmd.exec(),
        ManageRepositories::NewRepo(sub_cmd) => sub_cmd.exec(),
        ManageRepositories::Prune(sub_cmd) => sub_cmd.exec(),
    } {
        error!("{:?}", err);
        std::process::exit(1);
    }
}
