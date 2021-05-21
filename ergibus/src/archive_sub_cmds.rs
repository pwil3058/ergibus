// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>
use std::path::PathBuf;

use structopt::StructOpt;

use ergibus_lib::{archive, EResult};

#[derive(Debug, StructOpt)]
/// Manage snapshot archives
pub enum ManageArchives {
    /// Create a new snapshot archive.
    New {
        /// the name of the new snapshot archive to be created.
        #[structopt(short, long = "archive")]
        archive_name: String,
        /// the name of the repository that the new archive should use to store file contents.
        #[structopt(short = "r", long = "repo")]
        content_repo_name: String,
        /// the directory path of the location where the archive should store its snapshots.
        #[structopt(short, long = "location", parse(from_os_str))]
        location: PathBuf,
        /// the path of a file/directory that should be included in the archive's snapshots.
        #[structopt(short, long = "include", parse(from_os_str))]
        inclusions: Vec<PathBuf>,
        /// exclude directories matching this glob expression from patches.
        #[structopt(short, long = "exclude_dirs", required = false)]
        dir_exclusions: Vec<String>,
        /// exclude files matching this glob expression from patches.
        #[structopt(short, long = "exclude_files", required = false)]
        file_exclusions: Vec<String>,
    },
    /// List defined archives.
    List,
    /// Delete the specified archive
    #[structopt(alias = "del")]
    Delete {
        /// The name of the archive to be deleted
        archive_name: String,
    },
}

impl ManageArchives {
    pub fn exec(&self) -> EResult<()> {
        use ManageArchives::*;
        match self {
            New {
                archive_name,
                content_repo_name,
                location,
                inclusions,
                dir_exclusions,
                file_exclusions,
            } => {
                archive::create_new_archive(
                    archive_name,
                    content_repo_name,
                    location,
                    inclusions,
                    dir_exclusions,
                    file_exclusions,
                )?;
                Ok(())
            }
            List => {
                for archive_name in archive::get_archive_names() {
                    println!("{}", archive_name);
                }
                Ok(())
            }
            Delete { archive_name } => archive::delete_archive(archive_name),
        }
    }
}
