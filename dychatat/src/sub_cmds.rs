// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au> <pwil3058@outlook.com>
use std::path::PathBuf;

use structopt::StructOpt;

use dychatat_lib::{content, RepoResult};

#[derive(Debug, StructOpt)]
/// Manage content repositories
pub enum ManageRepositories {
    /// List repositories
    #[structopt(alias = "ls")]
    List(ListRepositories),
    /// Delete a repository
    #[structopt(alias = "del")]
    Delete(DeleteRepository),
    /// Prune a repository
    #[structopt(alias = "pr")]
    Prune(PruneRepository),
    /// Create a new repository
    #[structopt(alias = "new")]
    NewRepo(NewRepository),
}
//
// impl ManageRepositories {
//     pub fn exec(&self) -> RepoResult<()> {
//         use ManageRepositories::*;
//         match self {
//             List(sub_cmd) => sub_cmd.exec(),
//             Delete(sub_cmd) => sub_cmd.exec(),
//             Prune(sub_cmd) => sub_cmd.exec(),
//             NewRepo(sub_cmd) => sub_cmd.exec(),
//         }
//     }
// }

#[derive(Debug, StructOpt)]
/// List content repositories
pub struct ListRepositories {
    /// Show specification
    #[structopt(short, long)]
    show: bool,
}

impl ListRepositories {
    pub fn exec(&self) -> RepoResult<()> {
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
    pub fn exec(&self) -> RepoResult<()> {
        content::delete_repository(&self.repo_name)
    }
}

#[derive(Debug, StructOpt)]
/// Prune a content repository
pub struct PruneRepository {
    /// The name of the repository to be pruned
    repo_name: String,
}

impl PruneRepository {
    pub fn exec(&self) -> RepoResult<()> {
        let stats = content::prune_repository(&self.repo_name)?;
        println!("{:?}", stats);
        Ok(())
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
    pub fn exec(&self) -> RepoResult<()> {
        content::create_new_repo(&self.repo_name, &self.location, &self.algorithm)
    }
}
