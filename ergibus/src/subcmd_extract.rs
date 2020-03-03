use std::env;
use std::io::{stderr, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time;

// crates.io
use clap;
use structopt::StructOpt;

// github
use pw_pathux;

// local
use crate::cli;
use ergibus_lib::eerror::EResult;
use ergibus_lib::snapshot::{ArchiveOrDirPath, ExtractionStats, SnapshotPersistentData};

#[derive(Debug, StructOpt)]
#[structopt(group = clap::ArgGroup::with_name("which").required(true), group = clap::ArgGroup::with_name("what").required(true))]
pub struct Extract {
    /// select the snapshot "N" places before the most recent. Use -1 to select oldest.
    #[structopt(long, value_name = "N", default_value = "0")]
    back_n: i64,
    /// the name of the snapshot archive whose file or directory is to be extracted.
    #[structopt(short = "A", long = "archive", group = "which")]
    archive_name: Option<String>,
    /// the name of the directory containing the snapshots whose file or
    /// directory is to be extracted. This option is intended for use in those
    /// cases where the configuration data has been lost (possibly due to file
    /// system failure).  Individual snapshot files contain sufficient data for
    /// extraction of files or directories without the need for the
    /// configuration files provided their content repositories are also intact.
    #[structopt(short = "X", long = "exigency", group = "which", parse(from_os_str))]
    exigency_dir_path: Option<PathBuf>,
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
    #[structopt(long)]
    with_name: Option<PathBuf>,
    /// the path of the directory into which the file/directory is to be copied.
    #[structopt(long)]
    into_dir: Option<PathBuf>,
    /// show statistics for the extraction process.
    #[structopt(long)]
    stats: bool,
}

impl Extract {
    pub fn exec(&self) {
        let archive_or_dir_path = if let Some(archive_name) = &self.archive_name {
            ArchiveOrDirPath::Archive(archive_name.clone())
        } else if let Some(dir_path) = &self.exigency_dir_path {
            ArchiveOrDirPath::DirPath(dir_path.to_path_buf())
        } else {
            println!("either --archive or --exigency must be present");
            std::process::exit(1);
        };
        let into_dir_path = if let Some(into_dir) = &self.into_dir {
            into_dir.clone()
        } else {
            env::current_dir().unwrap()
        };
        if let Some(file_path) = &self.file_path {
            println!(
                "extract file: {:?} from: {:?}",
                file_path, archive_or_dir_path
            );
            match copy_file_to(
                &archive_or_dir_path,
                self.back_n,
                file_path,
                &into_dir_path,
                &self.with_name,
                self.overwrite,
            ) {
                Ok(stats) => {
                    if self.stats {
                        println!("Transfered {} bytes in {:?}", stats.0, stats.1)
                    }
                }
                Err(err) => {
                    writeln!(stderr(), "Error: {:?}", err).unwrap();
                    std::process::exit(1);
                }
            }
        } else if let Some(dir_path) = &self.dir_path {
            println!(
                "extract dir: {:?} from: {:?}",
                dir_path, archive_or_dir_path
            );
            match copy_dir_to(
                &archive_or_dir_path,
                self.back_n,
                dir_path,
                &into_dir_path,
                &self.with_name,
                self.overwrite,
            ) {
                Ok(stats) => {
                    if self.stats {
                        println!("Transfered {} files containing {} bytes and {} sym links in {} dirs in {:?}",
                                 stats.0.file_count,
                                 stats.0.bytes_count,
                                 (stats.0.dir_sym_link_count + stats.0.file_sym_link_count),
                                 stats.0.dir_count,
                                 stats.1
                        )
                    }
                }
                Err(err) => {
                    writeln!(stderr(), "Error: {:?}", err).unwrap();
                    std::process::exit(1);
                }
            }
        } else {
            println!("either --file or --dir must be present");
            std::process::exit(1);
        }
    }
}

pub fn sub_cmd<'a, 'b>() -> clap::App<'a, 'b> {
    clap::SubCommand::with_name("extract")
        .about(
            "Extract a copy of the nominated file/directory in the
nominated archive's most recent (or specified) snapshot
and place it in the current (or specified) directory.",
        )
        .arg(cli::arg_back_n().required(false))
        .arg(
            cli::arg_archive_name()
                .required(true)
                .help("the name of the archive whose file or directory is to be extracted"),
        )
        .arg(cli::arg_exigency_dir_path().help(
            "the name of the directory containing the snapshots whose file or
directory is to be extracted. This option is intended for use in those
cases where the configuration data has been lost (possibly due to file
system failure).  Individual snapshot files contain sufficient data for
extraction of files or directories without the need for the
configuration files provided their content repositories are also intact.",
        ))
        .group(
            clap::ArgGroup::with_name("which")
                .args(&["archive_name", "exigency_dir_path"])
                .required(true),
        )
        .arg(cli::arg_file_path().help("the path of the file to be copied."))
        .arg(cli::arg_dir_path().help("the path of the directory to be copied."))
        .group(
            clap::ArgGroup::with_name("what")
                .args(&["file_path", "dir_path"])
                .required(false),
        )
        .arg(
            cli::arg_show_stats()
                .required(false)
                .multiple(false)
                .help("show statistics for the extraction process"),
        )
        .arg(cli::arg_overwrite().required(false))
        .arg(
            clap::Arg::with_name("with_name")
                .long("with_name")
                .takes_value(true)
                .value_name("name")
                .required(false)
                .help("the name to be given to the copy of the file/directory."),
        )
        .arg(
            clap::Arg::with_name("into_dir")
                .long("into_dir")
                .takes_value(true)
                .value_name("path")
                .required(false)
                .help("the path of the directory into which the file/directory is to be copied."),
        )
}

pub fn run_cmd(arg_matches: &clap::ArgMatches<'_>) {
    let archive_or_dir_path = if let Some(archive_name) = arg_matches.value_of("archive_name") {
        ArchiveOrDirPath::Archive(archive_name.to_string())
    } else if let Some(dir_path) = arg_matches.value_of("exigency_dir_path") {
        ArchiveOrDirPath::DirPath(PathBuf::from(dir_path))
    } else {
        panic!("{:?}: line {:?}", file!(), line!())
    };
    let n: i64 = if let Some(back_n_as_str) = arg_matches.value_of("back_n") {
        match i64::from_str(back_n_as_str) {
            Ok(n) => n,
            Err(_) => {
                writeln!(stderr(), "Expected signed integer: found {}", back_n_as_str).unwrap();
                std::process::exit(1);
            }
        }
    } else {
        0
    };
    let into_dir_path = if let Some(ref text) = arg_matches.value_of("into_dir") {
        PathBuf::from(text)
    } else {
        env::current_dir().unwrap()
    };
    let opt_with_name = if let Some(ref text) = arg_matches.value_of("with_name") {
        Some(PathBuf::from(text))
    } else {
        None
    };
    let overwrite = arg_matches.is_present("overwrite");
    let show_stats = arg_matches.is_present("show_stats");
    if let Some(text) = arg_matches.value_of("file_path") {
        let file_path = PathBuf::from(&text);
        match copy_file_to(
            &archive_or_dir_path,
            n,
            &file_path,
            &into_dir_path,
            &opt_with_name,
            overwrite,
        ) {
            Ok(stats) => {
                if show_stats {
                    println!("Transfered {} bytes in {:?}", stats.0, stats.1)
                }
            }
            Err(err) => {
                writeln!(stderr(), "Error: {:?}", err).unwrap();
                std::process::exit(1);
            }
        }
    } else if let Some(text) = arg_matches.value_of("dir_path") {
        let dir_path = PathBuf::from(&text);
        match copy_dir_to(
            &archive_or_dir_path,
            n,
            &dir_path,
            &into_dir_path,
            &opt_with_name,
            overwrite,
        ) {
            Ok(stats) => {
                if show_stats {
                    println!("Transfered {} files containing {} bytes and {} synm links in {} dirs in {:?}",
                    stats.0.file_count,
                    stats.0.bytes_count,
                    (stats.0.dir_sym_link_count + stats.0.file_sym_link_count),
                    stats.0.dir_count,
                    stats.1
                )
                }
            }
            Err(err) => {
                writeln!(stderr(), "Error: {:?}", err).unwrap();
                std::process::exit(1);
            }
        }
    } else {
        panic!("{:?}: line {:?}", file!(), line!())
    }
}

fn copy_file_to(
    archive_or_dir_path: &ArchiveOrDirPath,
    n: i64,
    file_path: &Path,
    into_dir_path: &Path,
    opt_with_name: &Option<PathBuf>,
    overwrite: bool,
) -> EResult<(u64, time::Duration)> {
    let started_at = time::SystemTime::now();

    let snapshot_file_path = archive_or_dir_path.get_snapshot_path_back_n(n)?;
    let target_path = if let Some(with_name) = opt_with_name {
        into_dir_path.join(with_name)
    } else if let Some(file_name) = file_path.file_name() {
        into_dir_path.join(file_name)
    } else {
        panic!("{:?}: line {:?}", file!(), line!())
    };
    let abs_file_path = if file_path.starts_with("~") {
        pw_pathux::expand_home_dir(file_path).unwrap()
    } else {
        pw_pathux::absolute_path_buf(file_path)
    };
    let spd = SnapshotPersistentData::from_file(&snapshot_file_path)?;
    let bytes = spd.copy_file_to(&abs_file_path, &target_path, overwrite)?;

    let finished_at = time::SystemTime::now();
    let duration = match finished_at.duration_since(started_at) {
        Ok(duration) => duration,
        Err(_) => time::Duration::new(0, 0),
    };
    Ok((bytes, duration))
}

fn copy_dir_to(
    archive_or_dir_path: &ArchiveOrDirPath,
    n: i64,
    dir_path: &Path,
    into_dir_path: &Path,
    opt_with_name: &Option<PathBuf>,
    overwrite: bool,
) -> EResult<(ExtractionStats, time::Duration)> {
    let started_at = time::SystemTime::now();

    let snapshot_file_path = archive_or_dir_path.get_snapshot_path_back_n(n)?;
    let target_path = if let Some(with_name) = opt_with_name {
        into_dir_path.join(with_name)
    } else if let Some(dir_name) = dir_path.file_name() {
        into_dir_path.join(dir_name)
    } else {
        panic!("{:?}: line {:?}", file!(), line!())
    };
    let abs_dir_path = if dir_path.starts_with("~") {
        pw_pathux::expand_home_dir(dir_path).unwrap()
    } else {
        pw_pathux::absolute_path_buf(dir_path)
    };
    let spd = SnapshotPersistentData::from_file(&snapshot_file_path)?;
    let stats = spd.copy_dir_to(
        &abs_dir_path,
        &target_path,
        overwrite,
        &mut Some(&mut stderr()),
    )?;

    let finished_at = time::SystemTime::now();
    let duration = match finished_at.duration_since(started_at) {
        Ok(duration) => duration,
        Err(_) => time::Duration::new(0, 0),
    };
    Ok((stats, duration))
}
