// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>
use std::path::PathBuf;

use structopt::StructOpt;

use ergibus_lib::{content, EResult};

#[derive(Debug, StructOpt)]
/// Manage content repositories
pub enum ManageRepositories {
    /// List repositories
    #[structopt(alias = "ls")]
    List(ListRepositories),
    /// Delete a repository
    #[structopt(alias = "del")]
    Delete(DeleteRepository),
    /// Create a new repository
    #[structopt(alias = "new")]
    NewRepo(NewRepository),
}

impl ManageRepositories {
    pub fn exec(&self) -> EResult<()> {
        use ManageRepositories::*;
        match self {
            List(sub_cmd) => sub_cmd.exec(),
            Delete(sub_cmd) => sub_cmd.exec(),
            NewRepo(sub_cmd) => sub_cmd.exec(),
        }
    }
}

#[derive(Debug, StructOpt)]
/// List content repositories
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
/// Delete a content repository
pub struct DeleteRepository {
    /// The name of the repository to be deleted
    repo_name: String,
}

impl DeleteRepository {
    pub fn exec(&self) -> EResult<()> {
        content::delete_repository(&self.repo_name)
    }
}

const ALGORITHMS: &[&str] = &["Sha1", "Sha256", "Sha512"];

#[derive(Debug, StructOpt)]
/// Create a new content repository
pub struct NewRepository {
    /// The name of the new repository
    repo_name: String,
    /// The location of the base directory in which the repository is to be placed.
    #[structopt(short, long, parse(from_os_str))]
    location: PathBuf,
    /// The hash algorithm to use when generating repository's file content token
    #[structopt(short, long, possible_values(ALGORITHMS))]
    algorithm: String,
}

impl NewRepository {
    pub fn exec(&self) -> EResult<()> {
        content::create_new_repo(&self.repo_name, &self.location, &self.algorithm)
    }
}
