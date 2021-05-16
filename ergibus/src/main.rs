// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

mod repositories;

use structopt::StructOpt;

use repositories::ListRepositories;

#[derive(Debug, StructOpt)]
/// Experimental Rust Git Inspired Back Up System
enum Ergibus {
    /// List repositories
    #[structopt(alias = "lr")]
    LR(ListRepositories),
}

fn main() {
    let ergibus = Ergibus::from_args();

    if let Err(err) = match ergibus {
        Ergibus::LR(sub_cmd) => sub_cmd.exec(),
    } {
        println!("failed: {:?}", err);
    }
}
