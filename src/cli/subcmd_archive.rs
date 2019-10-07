use structopt::StructOpt;

use crate::archive;

#[derive(Debug, StructOpt)]
pub enum Archive {
    /// Create a new snapshot archive.
    New {
        /// the name of the new snapshot archive to be created.
        #[structopt(short = "A", long = "archive")]
        archive_name: String,
        /// the name of the repository that the new archive should use to store file contents.
        #[structopt(short = "R", long = "repo")]
        content_repo_name: String,
        /// the directory path of the location where the archive should store its snapshots.
        #[structopt(short = "L", long = "location")]
        location: String,
        /// the path of a file/directory that should be included in the archive's snapshots.
        #[structopt(short = "I", long = "include")]
        inclusions: Vec<String>,
        /// exclude directories matching this glob expression from patches.
        #[structopt(short = "D", long = "exclude_dirs", required = false)]
        dir_exclusions: Vec<String>,
        /// exclude files matching this glob expression from patches.
        #[structopt(short = "F", long = "exclude_files", required = false)]
        file_exclusions: Vec<String>,
    },
    /// List defined archives.
    List,
}

impl Archive {
    pub fn exec(&self) {
        match self {
            Archive::New {
                archive_name,
                content_repo_name,
                location,
                inclusions,
                dir_exclusions,
                file_exclusions,
            } => {
                if let Err(err) = archive::create_new_archive(
                    archive_name,
                    content_repo_name,
                    location,
                    inclusions,
                    dir_exclusions,
                    file_exclusions,
                ) {
                    println!("{:?}", err);
                    std::process::exit(1);
                };
            }
            Archive::List => {
                for archive_name in archive::get_archive_names() {
                    println!("{}", archive_name);
                }
            }
        }
    }
}
