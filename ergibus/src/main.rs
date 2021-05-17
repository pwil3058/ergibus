// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

mod repositories;

use structopt::StructOpt;

use crate::repositories::{DeleteRepository, ListRepositories, NewRepository};

#[derive(Debug, StructOpt)]
/// Experimental Rust Git Inspired Back Up System
enum Ergibus {
    /// List repositories
    #[structopt(alias = "lr")]
    LR(ListRepositories),
    /// Delete a repository
    #[structopt(alias = "dr")]
    DR(DeleteRepository),
    /// Delete a repository
    #[structopt(alias = "nr")]
    NewR(NewRepository),
}

fn main() {
    let ergibus = Ergibus::from_args();

    if let Err(err) = match ergibus {
        Ergibus::LR(sub_cmd) => sub_cmd.exec(),
        Ergibus::DR(sub_cmd) => sub_cmd.exec(),
        Ergibus::NewR(sub_cmd) => sub_cmd.exec(),
    } {
        println!("failed: {:?}", err);
    }
}
