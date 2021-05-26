// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::archive::{get_archive_data, ArchiveData};
use crate::content::ContentMgmtKey;
use crate::fs_objects::DirectoryData;
use crate::snapshot::{FileStats, SymLinkStats};
use crate::{EResult, Error, UNEXPECTED};
use chrono::{DateTime, Local};
use serde::Serialize;
use std::convert::TryFrom;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::{fs, time};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct SnapshotPersistentData {
    root_dir: DirectoryData,
    base_dir_path: PathBuf,
    content_mgmt_key: ContentMgmtKey,
    archive_name: String,
    started_create: time::SystemTime,
    finished_create: time::SystemTime,
    file_stats: FileStats,
    sym_link_stats: SymLinkStats,
}

impl TryFrom<&ArchiveData> for SnapshotPersistentData {
    type Error = Error;

    fn try_from(archive_data: &ArchiveData) -> EResult<Self> {
        let root_dir = DirectoryData::new(Component::RootDir)?;
        let base_dir_path = root_dir.path.clone();
        Ok(Self {
            root_dir,
            base_dir_path,
            content_mgmt_key: archive_data.content_mgmt_key.clone(),
            archive_name: archive_data.name.clone(),
            started_create: time::SystemTime::now(),
            finished_create: time::SystemTime::now(),
            file_stats: FileStats::default(),
            sym_link_stats: SymLinkStats::default(),
        })
    }
}

impl SnapshotPersistentData {
    fn serialize(&self) -> EResult<String> {
        match serde_json::to_string(self) {
            Ok(string) => Ok(string),
            Err(err) => Err(Error::SnapshotSerializeError(err)),
        }
    }

    fn release_contents(&self) -> EResult<()> {
        let content_mgr = self
            .content_mgmt_key
            .open_content_manager(dychatat::Mutability::Mutable)?;
        self.root_dir.release_contents(&content_mgr)
    }

    fn snapshot_name(&self) -> String {
        let dt = DateTime::<Local>::from(self.finished_create);
        format!("{}", dt.format("%Y-%m-%d-%H-%M-%S%z"))
    }

    fn write_to_dir<P: AsRef<Path>>(&self, dir_path: P) -> EResult<PathBuf> {
        let file_name = self.snapshot_name();
        let path = dir_path.as_ref().join(file_name);
        let file = File::create(&path)
            .map_err(|err| Error::SnapshotWriteIOError(err, path.to_path_buf()))?;
        let json_text = self.serialize()?;
        let mut snappy_wtr = snap::Writer::new(file);
        snappy_wtr
            .write_all(json_text.as_bytes())
            .map_err(|err| Error::SnapshotWriteIOError(err, path.to_path_buf()))?;
        Ok(path)
    }
}

// Doing this near where the file names are constructed for programming convenience
lazy_static! {
    static ref SS_FILE_NAME_RE: regex::Regex =
        regex::Regex::new(r"^(\d{4})-(\d{2})-(\d{2})-(\d{2})-(\d{2})-(\d{2})[+-](\d{4})$").unwrap();
}

impl SnapshotPersistentData {
    // Interrogation/extraction/restoration methods

    pub fn from_file<P: AsRef<Path>>(file_path_arg: P) -> EResult<SnapshotPersistentData> {
        let file_path = file_path_arg.as_ref();
        match File::open(file_path) {
            Ok(file) => {
                let mut spd_str = String::new();
                let mut snappy_rdr = snap::Reader::new(file);
                match snappy_rdr.read_to_string(&mut spd_str) {
                    Err(err) => {
                        return Err(Error::SnapshotReadIOError(err, file_path.to_path_buf()))
                    }
                    _ => (),
                };
                let spde = serde_json::from_str::<SnapshotPersistentData>(&spd_str);
                match spde {
                    Ok(snapshot_persistent_data) => Ok(snapshot_persistent_data),
                    Err(err) => Err(Error::SnapshotReadJsonError(err, file_path.to_path_buf())),
                }
            }
            Err(err) => Err(Error::SnapshotReadIOError(err, file_path.to_path_buf())),
        }
    }
}

#[derive(Debug)]
struct SnapshotGenerator {
    snapshot: Option<SnapshotPersistentData>,
    archive_data: ArchiveData,
}

impl Drop for SnapshotGenerator {
    fn drop(&mut self) {
        if self.snapshot.is_some() {
            self.release_snapshot().expect(UNEXPECTED);
        }
    }
}

impl SnapshotGenerator {
    pub fn new(archive_name: &str) -> EResult<SnapshotGenerator> {
        let archive_data = get_archive_data(archive_name)?;
        // Check that there'll be no problem starting the creation of snapshots
        let _dummy = SnapshotPersistentData::try_from(&archive_data)?;
        Ok(SnapshotGenerator {
            snapshot: None,
            archive_data,
        })
    }

    #[cfg(test)]
    pub fn snapshot_available(&self) -> bool {
        self.snapshot.is_some()
    }

    // fn generate_snapshot(&mut self) -> EResult<(time::Duration, FileStats, SymLinkStats, u64)> {
    //     if self.snapshot.is_some() {
    //         // This snapshot is being thrown away so we release its contents
    //         self.release_snapshot();
    //     }
    //     let mut delta_repo_size: u64 = 0;
    //     let mut snapshot = SnapshotPersistentData::try_from(&self.archive_data)?;
    //     for abs_path in self.archive_data.includes.iter() {
    //         if abs_path.is_dir() {
    //             match snapshot.add_dir(&abs_path, &self.archive_data.exclusions) {
    //                 Ok(drsz) => delta_repo_size += drsz,
    //                 Err(err) => match err.kind() {
    //                     ErrorKind::NotFound => warn!("{:?}: not found", abs_path),
    //                     _ => ignore_report_or_crash(&err, &abs_path),
    //                 },
    //             };
    //         } else {
    //             match snapshot.add_other(&abs_path) {
    //                 Ok(drsz) => delta_repo_size += drsz,
    //                 Err(err) => match err.kind() {
    //                     ErrorKind::NotFound => warn!("{:?}: not found", abs_path),
    //                     _ => ignore_report_or_crash(&err, &abs_path),
    //                 },
    //             };
    //         }
    //     }
    //     snapshot.finished_create = time::SystemTime::now();
    //     let duration = snapshot.creation_duration();
    //     let file_stats = snapshot.file_stats;
    //     let sym_link_stats = snapshot.sym_link_stats;
    //     self.snapshot = Some(snapshot);
    //     Ok((duration, file_stats, sym_link_stats, delta_repo_size))
    // }

    #[cfg(test)]
    pub fn generation_duration(&self) -> EResult<time::Duration> {
        match self.snapshot {
            Some(ref snapshot) => Ok(snapshot.creation_duration()),
            None => Err(Error::NoSnapshotAvailable),
        }
    }

    fn release_snapshot(&mut self) -> EResult<()> {
        match self.snapshot {
            Some(ref snapshot) => snapshot.release_contents()?,
            None => (),
        }
        self.snapshot = None;
        Ok(())
    }

    fn write_snapshot(&mut self) -> EResult<PathBuf> {
        let file_path = match self.snapshot {
            Some(ref snapshot) => snapshot.write_to_dir(&self.archive_data.snapshot_dir_path)?,
            None => return Err(Error::NoSnapshotAvailable),
        };
        // check that the snapshot can be rebuilt from the file
        match SnapshotPersistentData::from_file(&file_path) {
            Ok(rb_snapshot) => {
                if self.snapshot == Some(rb_snapshot) {
                    // don't release contents as references are stored in the file
                    self.snapshot = None;
                    Ok(file_path)
                } else {
                    // The file is mangled so remove it
                    match fs::remove_file(&file_path) {
                        Ok(_) => Err(Error::SnapshotMismatch(file_path.to_path_buf())),
                        Err(err) => Err(Error::SnapshotMismatchDirty(err, file_path.to_path_buf())),
                    }
                }
            }
            Err(err) => {
                // The file is mangled so remove it
                match fs::remove_file(&file_path) {
                    Ok(_) => Err(err),
                    Err(_) => Err(err),
                }
            }
        }
    }
}
