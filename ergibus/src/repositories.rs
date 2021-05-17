// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>
use structopt::StructOpt;

use ergibus_lib::{content, EResult};

#[derive(Debug, StructOpt)]
/// List repositories
pub struct ListRepositories {
    /// Show specification
    #[structopt(short, long)]
    show: bool,
}

impl ListRepositories {
    pub fn exec(&self) -> EResult<()> {
        for repo_name in content::get_repo_names() {
            if self.show {
                let spec = content::read_repo_spec(&repo_name)?;
                println!("{}: {}", repo_name, spec)
            } else {
                println!("{}", repo_name)
            }
        }
        Ok(())
    }
}

#[derive(Debug, StructOpt)]
/// Delete a repository
pub struct DeleteRepository {
    /// The name of the repository to be deleted
    repo_name: String,
}

impl DeleteRepository {
    pub fn exec(&self) -> EResult<()> {
        content::delete_repository(&self.repo_name)
    }
}
