// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::convert::TryFrom;
use std::path::PathBuf;

use structopt::{clap::ArgGroup, StructOpt};

use ergibus_lib::{archive::Snapshots, snapshot, EResult, Error};
use std::env;

#[derive(Debug, StructOpt)]
#[structopt(group = ArgGroup::with_name("which").required(true))]
pub struct SnapshotManager {
    /// the name of the snapshot archive that contains the snapshot(s) to be acted on.
    #[structopt(short, long = "archive", group = "which")]
    archive_name: Option<String>,
    /// the name of the directory containing the snapshot(s) to be acted on.
    ///
    /// This option is intended for use in those
    /// cases where the configuration data has been lost (possibly due to file
    /// system failure).  Individual snapshot files contain sufficient data for
    /// extraction of files or directories without the need for the
    /// configuration files provided their content repositories are also intact.
    #[structopt(short = "x", long = "exigency", group = "which", parse(from_os_str))]
    exigency_dir_path: Option<PathBuf>,
    #[structopt(subcommand)]
    sub_cmd: SubCmd,
}

#[derive(Debug, StructOpt)]
pub enum SubCmd {
    /// List the snapshots for a nominated archive (or in a nominated directory).
    List,
    /// Delete the specified snapshot(s).
    #[structopt(alias = "del", group = ArgGroup::with_name("which_ss").required(true))]
    Delete {
        /// all but newest `N` snapshots.
        #[structopt(short, long, value_name = "N", group = "which_ss")]
        all_but_newest_n: Option<usize>,
        /// delete the snapshot "N" places before the most recent. Use -1 to select oldest.
        #[structopt(short, long, value_name = "N", group = "which_ss")]
        back_n: Option<i64>,
        /// authorise deletion of the last remaining snapshot in the archive.
        #[structopt(short, long)]
        clear_fell: bool,
        /// Verbose: report the number of snapshots deleted.
        #[structopt(short, long)]
        verbose: bool,
    },
}

impl SnapshotManager {
    pub fn exec(&self) -> EResult<()> {
        let snapshot_dir = if let Some(archive_name) = &self.archive_name {
            Snapshots::try_from(archive_name.as_str())?
        } else if let Some(dir_path) = &self.exigency_dir_path {
            Snapshots::try_from(dir_path.as_path())?
        } else {
            panic!("either --archive or --exigency must be present");
        };
        match self.sub_cmd {
            SubCmd::List => {
                for name in snapshot_dir.get_snapshot_names(false)?.iter() {
                    println!("{:?}", name);
                }
            }
            SubCmd::Delete {
                all_but_newest_n,
                back_n,
                clear_fell,
                verbose,
            } => {
                let number = if let Some(count) = all_but_newest_n {
                    snapshot_dir.delete_all_but_newest(count, clear_fell)?
                } else if let Some(back_n) = back_n {
                    snapshot_dir.delete_ss_back_n(back_n, clear_fell)?
                } else {
                    panic!("clap shouldn't let us get here")
                };
                if verbose {
                    println!("{} snapshots deleted.", number)
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, StructOpt)]
#[structopt(group = ArgGroup::with_name("which").required(true))]
pub struct SnapshotContents {
    /// the name of the snapshot archive that contains the snapshot to be acted on.
    #[structopt(short, long = "archive", group = "which")]
    archive_name: Option<String>,
    /// the name of the directory containing the snapshot to be acted on.
    ///
    /// This option is intended for use in those
    /// cases where the configuration data has been lost (possibly due to file
    /// system failure).  Individual snapshot files contain sufficient data for
    /// extraction of files or directories without the need for the
    /// configuration files provided their content repositories are also intact.
    #[structopt(short = "x", long = "exigency", group = "which", parse(from_os_str))]
    exigency_dir_path: Option<PathBuf>,
    /// use the snapshot "N" places before the most recent. Use -1 to select oldest.
    #[structopt(short, long, value_name = "N", group = "which_ss")]
    back_n: i64,
    #[structopt(subcommand)]
    sub_cmd: ContentsSubCmd,
}

#[derive(Debug, StructOpt)]
pub enum ContentsSubCmd {
    /// Extract a file or directory from the specified snapshot
    #[structopt(group = ArgGroup::with_name("what").required(true))]
    Extract {
        /// the path of the file to be copied.
        #[structopt(
            short = "F",
            long = "file",
            value_name = "path",
            group = "what",
            parse(from_os_str)
        )]
        file_path: Option<PathBuf>,
        /// the path of the directory to be copied.
        #[structopt(
            short = "D",
            long = "dir",
            value_name = "path",
            group = "what",
            parse(from_os_str)
        )]
        dir_path: Option<PathBuf>,
        /// overwrite the file/directory if it already exists instead of moving it aside.
        #[structopt(long)]
        overwrite: bool,
        /// the name to be given to the copy of the file/directory.
        #[structopt(long, value_name = "path")]
        with_name: Option<PathBuf>,
        /// the path of the directory into which the file/directory is to be copied.
        #[structopt(long, value_name = "path")]
        into_dir: Option<PathBuf>,
        /// show statistics for the extraction process.
        #[structopt(long = "stats")]
        show_stats: bool,
    },
}

impl SnapshotContents {
    pub fn exec(&self) -> EResult<()> {
        let snapshot_dir = if let Some(archive_name) = &self.archive_name {
            Snapshots::try_from(archive_name.as_str())?
        } else if let Some(dir_path) = &self.exigency_dir_path {
            Snapshots::try_from(dir_path.as_path())?
        } else {
            panic!("either --archive or --exigency must be present");
        };
        use ContentsSubCmd::*;
        match &self.sub_cmd {
            Extract {
                file_path,
                dir_path,
                overwrite,
                with_name,
                into_dir,
                show_stats,
            } => {
                let into_dir = if let Some(into_dir) = into_dir {
                    into_dir.clone()
                } else {
                    env::current_dir()?
                };
                if let Some(file_path) = file_path {
                    let stats = snapshot_dir.copy_file_to(
                        self.back_n,
                        file_path,
                        &into_dir,
                        with_name,
                        *overwrite,
                    )?;
                    if *show_stats {
                        println!("Transfered {} bytes in {:?}", stats.0, stats.1)
                    }
                } else if let Some(dir_path) = dir_path {
                    let stats = snapshot_dir.copy_dir_to(
                        self.back_n,
                        dir_path,
                        &into_dir,
                        with_name,
                        *overwrite,
                    )?;
                    if *show_stats {
                        println!("Transfered {} files containing {} bytes and {} sym links in {} dirs in {:?}", 
                                 stats.0.file_count,
                                 stats.0.bytes_count,
                                 (stats.0.dir_sym_link_count + stats.0.file_sym_link_count),
                                 stats.0.dir_count,
                                 stats.1
                        )
                    }
                } else {
                    panic!("clap shouldn't have let us get here")
                };
                Ok(())
            }
        }
    }
}

#[derive(Debug, StructOpt)]
pub struct BackUp {
    /// Show statistics for the generated snapshots.
    #[structopt(long = "stats")]
    show_stats: bool,
    /// Names of archives for which back ups are to be made
    #[structopt(required(true))]
    archives: Vec<String>,
}

impl BackUp {
    pub fn exec(&self) -> EResult<()> {
        let mut error_count = 0;
        if self.show_stats {
            println!(
                "{:>12} | {:>12} | {:>12} | {:>12} | {:>8} | {:>8} | {:>14} | {}",
                "#Files",
                "#Bytes",
                "#Stored",
                "#Change",
                "#Dir SL",
                "#File SL",
                "Time taken",
                "Archive Name"
            );
        };
        for archive in self.archives.iter() {
            match snapshot::generate_snapshot(&archive) {
                Ok(stats) => {
                    if self.show_stats {
                        let time_taken = format!("{:?}", stats.0);
                        println!(
                            "{:>12} | {:>12} | {:>12} | {:>12} | {:>8} | {:>8} | {:>14} | {}",
                            stats.1.file_count,
                            stats.1.byte_count,
                            stats.1.stored_byte_count,
                            stats.3,
                            stats.2.dir_sym_link_count,
                            stats.2.file_sym_link_count,
                            time_taken,
                            archive,
                        );
                    }
                }
                Err(err) => {
                    println!("{:?}: {}", err, archive);
                    error_count += 1;
                }
            }
        }
        if error_count > 0 {
            Err(Error::SnapshotsFailed(error_count))
        } else {
            Ok(())
        }
    }
}
