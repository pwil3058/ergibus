// Copyright 2019 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>
use structopt::StructOpt;

use ergibus_lib::content;

#[derive(Debug, StructOpt)]
pub enum Repository {
    /// Create a new content repository.
    New {
        /// the name of the content repository to be created.
        #[structopt(short = "R", long = "repo")]
        repo_name: String,
        /// the directory path of the location where the repository should store content.
        #[structopt(short = "L", long = "location")]
        location: String,
        /// the hash algorithm to use when generating repository's file content tokens
        #[structopt(short = "T", long = "token_hash_algorithm", default_value = "Sha256", possible_values = &["Sha1", "Sha256", "Sha512"])]
        algorithm: String,
    },
    /// List defined content repositories.
    List,
}

impl Repository {
    pub fn exec(&self) {
        match self {
            Repository::New {
                repo_name,
                location,
                algorithm,
            } => {
                if let Err(err) = content::create_new_repo(repo_name, location, algorithm) {
                    println!("{:?}", err);
                    std::process::exit(1);
                };
            }
            Repository::List => {
                for repo_name in content::get_repo_names() {
                    println!("{}", repo_name);
                }
            }
        }
    }
}
