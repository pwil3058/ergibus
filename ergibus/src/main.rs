// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

mod repositories;

use structopt::StructOpt;

use crate::repositories::ManageRepositories;

#[derive(Debug, StructOpt)]
/// Experimental Rust Git Inspired Back Up System
enum Ergibus {
    /// Manage repositories
    #[structopt(alias = "r")]
    Repo(ManageRepositories),
}

fn main() {
    let ergibus = Ergibus::from_args();

    if let Err(err) = match ergibus {
        Ergibus::Repo(sub_cmd) => sub_cmd.exec(),
    } {
        println!("failed: {:?}", err);
        std::process::exit(1);
    }
}
