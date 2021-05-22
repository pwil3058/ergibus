// Copyright 2019 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>
use std::path::PathBuf;
use structopt::StructOpt;

use ergibus_lib::archive::Snapshots;
use std::convert::TryFrom;

#[derive(Debug, StructOpt)]
pub struct Snapshot {
    /// the name of the snapshot archive that contains the snapshot(s) to be acted on.
    #[structopt(short = "A", long = "archive", group = "which")]
    archive_name: Option<String>,
    /// the name of the directory containing the snapshot(s) to be acted on.
    /// This option is intended for use in those
    /// cases where the configuration data has been lost (possibly due to file
    /// system failure).  Individual snapshot files contain sufficient data for
    /// extraction of files or directories without the need for the
    /// configuration files provided their content repositories are also intact.
    #[structopt(short = "X", long = "exigency", group = "which", parse(from_os_str))]
    exigency_dir_path: Option<PathBuf>,
    #[structopt(subcommand)]
    sub_cmd: SubCmd,
}

#[derive(Debug, StructOpt)]
pub enum SubCmd {
    /// List the snapshots for a nominated archive (or in a nominated directory).
    List,
    /// Delete the specified snapshot(s).
    Delete(Delete),
}

#[derive(Debug, StructOpt)]
#[structopt(group = clap::ArgGroup::with_name("which_ss").required(true))]
pub struct Delete {
    /// all but newest `N` snapshots.
    #[structopt(value_name = "N", group = "which_ss")]
    all_but_newest_n: Option<usize>,
    /// delete the snapshot "N" places before the most recent. Use -1 to select oldest.
    #[structopt(value_name = "N", group = "which_ss")]
    back_n: Option<i64>,
    /// authorise deletion of the last remaining snapshot in the archive.
    #[structopt(short, long)]
    clear_fell: bool,
    /// authorise deletion of the last remaining snapshot in the archive.
    #[structopt(short, long)]
    verbose: bool,
}

impl Delete {
    pub fn exec(&self, snapshot_dir: &Snapshots) {
        if let Some(count) = self.all_but_newest_n {
            match snapshot_dir.delete_all_but_newest(count, self.clear_fell) {
                Ok(number) => {
                    if self.verbose {
                        println!("{} snapshots deleted.", number)
                    }
                }
                Err(err) => {
                    // TODO: send error messages to stderr
                    println!("{:?}", err);
                    std::process::exit(1);
                }
            }
        } else if let Some(back_n) = self.back_n {
            match snapshot_dir.delete_ss_back_n(back_n, self.clear_fell) {
                Ok(number) => {
                    if self.verbose {
                        println!("{} snapshots deleted.", number)
                    }
                }
                Err(err) => {
                    // TODO: send error messages to stderr
                    println!("{:?}", err);
                    std::process::exit(1);
                }
            }
        } else {
            panic!("clap shouldn't le us get here")
        }
    }
}

impl Snapshot {
    pub fn exec(&self) {
        let snapshot_dir = if let Some(archive_name) = &self.archive_name {
            Snapshots::try_from(archive_name.as_str()).expect("no bad names")
        } else if let Some(dir_path) = &self.exigency_dir_path {
            Snapshots::try_from(dir_path.as_path()).expect("no bad names")
        } else {
            println!("either --archive or --exigency must be present");
            std::process::exit(1);
        };
        match self.sub_cmd {
            SubCmd::List => match snapshot_dir.get_snapshot_names(false) {
                Ok(snapshot_names) => {
                    for name in snapshot_names {
                        println!("{:?}", name);
                    }
                }
                Err(err) => {
                    println!("{:?}", err);
                    std::process::exit(1);
                }
            },
            SubCmd::Delete(ref delete) => delete.exec(&snapshot_dir),
        }
    }
}
