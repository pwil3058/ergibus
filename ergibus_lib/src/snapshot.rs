// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::convert::TryFrom;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{self, ErrorKind, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;
use std::{fs, time};

use chrono::{DateTime, Local};
use log::*;
use path_ext::{absolute_path_buf, PathType};
use path_utilities::UsableDirEntry;
use serde::Serialize;
use window_sort_iterator::WindowSortIterExt;

use crate::archive::{get_archive_data, ArchiveData, Exclusions};
use crate::content::ContentMgmtKey;
use crate::fs_objects::{DirectoryData, ExtractionStats, FileData, SymLinkData};
use crate::fs_objects::{FileStats, SymLinkStats};
use crate::report::ignore_report_or_fail;
use crate::{archive, EResult, Error, UNEXPECTED};

fn get_entry_for_path<P: AsRef<Path>>(path_arg: P) -> EResult<fs::DirEntry> {
    let path = path_arg.as_ref();
    if let Some(parent_dir_path) = path.parent() {
        let read_dir = fs::read_dir(&parent_dir_path)?;
        for entry in read_dir.filter_map(|e| e.ok()) {
            if entry.path() == path {
                return Ok(entry);
            }
        }
    }
    let io_error = io::Error::new(io::ErrorKind::NotFound, format!("{:?}: not found", path));
    Err(io_error.into())
}

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
        let root_dir = DirectoryData::try_new(Component::RootDir)?;
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

    fn add_dir(&mut self, abs_dir_path: &Path, exclusions: &Exclusions) -> EResult<u64> {
        let dir = self.root_dir.find_or_add_subdir(&abs_dir_path)?;
        let content_mgr = self
            .content_mgmt_key
            .open_content_manager(dychatat::Mutability::Mutable)?;
        let (file_stats, sym_link_stats, delta_repo_size) =
            dir.populate(exclusions, &content_mgr)?;
        self.file_stats += file_stats;
        self.sym_link_stats += sym_link_stats;
        Ok(delta_repo_size)
    }

    fn add_other(&mut self, abs_file_path: &Path) -> EResult<u64> {
        let entry = get_entry_for_path(abs_file_path)?;
        let dir_path = abs_file_path.parent().expect(UNEXPECTED);
        let dir = self.root_dir.find_or_add_subdir(&dir_path)?;
        let mut delta_repo_size: u64 = 0;
        match entry.file_type() {
            Ok(e_type) => match dir.index_for(&abs_file_path.file_name().expect(UNEXPECTED)) {
                Ok(_) => (),
                Err(index) => {
                    if e_type.is_file() {
                        let content_mgr = self
                            .content_mgmt_key
                            .open_content_manager(dychatat::Mutability::Mutable)?;
                        match FileData::file_system_object(abs_file_path, &content_mgr) {
                            Ok((file_system_object, stats, delta)) => {
                                self.file_stats += stats;
                                delta_repo_size = delta;
                                dir.contents.insert(index, file_system_object);
                            }
                            Err(err) => ignore_report_or_fail(err.into(), abs_file_path)?,
                        }
                    } else if e_type.is_symlink() {
                        match SymLinkData::file_system_object(abs_file_path) {
                            Ok((file_system_object, stats)) => {
                                self.sym_link_stats += stats;
                                dir.contents.insert(index, file_system_object);
                            }
                            Err(err) => ignore_report_or_fail(err.into(), abs_file_path)?,
                        }
                    }
                }
            },
            Err(err) => ignore_report_or_fail(err.into(), abs_file_path)?,
        };
        Ok(delta_repo_size)
    }

    fn add<P: AsRef<Path>>(&mut self, path_arg: P, exclusions: &Exclusions) -> EResult<u64> {
        if path_arg.as_ref().symlink_metadata()?.file_type().is_dir() {
            self.add_dir(path_arg.as_ref(), exclusions)
        } else {
            self.add_other(path_arg.as_ref())
        }
    }

    fn creation_duration(&self) -> time::Duration {
        match self.finished_create.duration_since(self.started_create) {
            Ok(duration) => duration,
            Err(_) => time::Duration::new(0, 0),
        }
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

    pub fn archive_name(&self) -> &str {
        &self.archive_name
    }

    pub fn base_dir_path(&self) -> &Path {
        self.base_dir_path.as_path()
    }

    pub fn root_dir_path(&self) -> &Path {
        self.root_dir.path()
    }

    pub fn content_mgmt_key(&self) -> &ContentMgmtKey {
        &self.content_mgmt_key
    }

    pub fn find_subdir<P: AsRef<Path>>(&self, dir_path_arg: P) -> EResult<&DirectoryData> {
        let dir_path = dir_path_arg.as_ref();
        match PathType::of(dir_path) {
            PathType::Absolute => self.root_dir.find_subdir(dir_path),
            PathType::RelativeCurDirImplicit => self
                .root_dir
                .find_subdir(&self.base_dir_path.join(dir_path)),
            PathType::Empty => self.root_dir.find_subdir(&self.base_dir_path),
            _ => self.root_dir.find_subdir(
                absolute_path_buf(dir_path)
                    .map_err(|_| Error::SnapshotUnknownDirectory(dir_path.to_path_buf()))?,
            ),
        }
    }

    pub fn find_file<P: AsRef<Path>>(&self, file_path_arg: P) -> EResult<&FileData> {
        let file_path = file_path_arg.as_ref();
        match PathType::of(file_path) {
            PathType::Absolute => self.root_dir.find_file(file_path),
            PathType::RelativeCurDirImplicit => {
                self.root_dir.find_file(&self.base_dir_path.join(file_path))
            }
            PathType::Empty => Err(Error::SnapshotUnknownFile(file_path.to_path_buf())),
            _ => self.root_dir.find_file(
                absolute_path_buf(file_path)
                    .map_err(|_| Error::SnapshotUnknownFile(file_path.to_path_buf()))?,
            ),
        }
    }

    pub fn copy_file_to(
        &self,
        fm_file_path: &Path,
        to_file_path: &Path,
        overwrite: bool,
    ) -> EResult<u64> {
        let file_data = self.find_file(fm_file_path)?;
        let c_mgr = self
            .content_mgmt_key
            .open_content_manager(dychatat::Mutability::Immutable)?;
        Ok(file_data.copy_contents_to(to_file_path, &c_mgr, overwrite)?)
    }

    pub fn copy_dir_to(
        &self,
        fm_dir_path: &Path,
        to_dir_path: &Path,
        overwrite: bool,
    ) -> EResult<ExtractionStats> {
        let fm_subdir = self.find_subdir(fm_dir_path)?;
        let stats = fm_subdir.copy_to(to_dir_path, &self.content_mgmt_key, overwrite)?;
        Ok(stats)
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

    fn generate_snapshot(&mut self) -> EResult<(time::Duration, FileStats, SymLinkStats, u64)> {
        if self.snapshot.is_some() {
            // This snapshot is being thrown away so we release its contents
            self.release_snapshot()?;
        }
        let mut delta_repo_size: u64 = 0;
        let mut snapshot = SnapshotPersistentData::try_from(&self.archive_data)?;
        for abs_path in self.archive_data.includes.iter() {
            match snapshot.add(abs_path, &self.archive_data.exclusions) {
                Ok(drsz) => delta_repo_size += drsz,
                Err(err) => match err {
                    Error::IOError(io_err) => match io_err.kind() {
                        ErrorKind::NotFound | ErrorKind::PermissionDenied => {
                            // non fatal errors so report and soldier on
                            warn!("{:?}: {:?}", abs_path, io_err)
                        }
                        _ => {
                            snapshot.release_contents()?;
                            return Err(io_err.into());
                        }
                    },
                    _ => {
                        snapshot.release_contents()?;
                        return Err(err);
                    }
                },
            };
        }
        let mut base_dir = &snapshot.root_dir;
        while base_dir.contents.len() == 1 {
            if let Some(subdir) = base_dir.subdirs().next() {
                base_dir = subdir
            } else {
                break;
            }
        }
        snapshot.base_dir_path = base_dir.path.to_path_buf();
        snapshot.finished_create = time::SystemTime::now();
        let duration = snapshot.creation_duration();
        let file_stats = snapshot.file_stats;
        let sym_link_stats = snapshot.sym_link_stats;
        self.snapshot = Some(snapshot);
        Ok((duration, file_stats, sym_link_stats, delta_repo_size))
    }

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
                    _ => Err(err),
                }
            }
        }
    }
}

pub fn generate_snapshot(
    archive_name: &str,
) -> EResult<(time::Duration, FileStats, SymLinkStats, u64)> {
    let mut sg = SnapshotGenerator::new(archive_name)?;
    let stats = sg.generate_snapshot()?;
    sg.write_snapshot()?;
    Ok(stats)
}

pub fn delete_snapshot_file(ss_file_path: &Path) -> EResult<()> {
    let snapshot = SnapshotPersistentData::from_file(ss_file_path)?;
    fs::remove_file(ss_file_path)
        .map_err(|err| Error::SnapshotDeleteIOError(err, ss_file_path.to_path_buf()))?;
    snapshot.release_contents()?;
    Ok(())
}

// Doing this near where the file names are constructed for programming convenience
lazy_static! {
    static ref SS_FILE_NAME_RE: regex::Regex =
        regex::Regex::new(r"^(\d{4})-(\d{2})-(\d{2})-(\d{2})-(\d{2})-(\d{2})[+-](\d{4})$").unwrap();
}

#[derive(Debug)]
pub enum Order {
    Ascending,
    Descending,
}
impl Order {
    pub fn is_ascending(&self) -> bool {
        match self {
            Order::Ascending => true,
            Order::Descending => false,
        }
    }

    pub fn is_descending(&self) -> bool {
        match self {
            Order::Ascending => false,
            Order::Descending => true,
        }
    }
}

fn iter_snapshot_i_in_dir<'a, I: Ord + 'a>(
    dir_path: PathBuf,
    order: Order,
    ude_to_i: fn(UsableDirEntry) -> I,
) -> EResult<Box<dyn Iterator<Item = I> + 'a>> {
    let iter = path_utilities::usable_dir_entries(&dir_path)
        .map_err(|err| Error::SnapshotDirIOError(err, dir_path.to_path_buf()))?
        .filter(|e| e.is_file() && SS_FILE_NAME_RE.is_match(&e.file_name().to_string_lossy()))
        .map(move |e| ude_to_i(e));
    match order {
        Order::Ascending => Ok(Box::new(
            iter.map(|e| std::cmp::Reverse(e))
                .window_sort(usize::MAX)
                .map(|e| e.0),
        )),
        Order::Descending => Ok(Box::new(iter.window_sort(usize::MAX))),
    }
}

pub fn iter_snapshot_names_in_dir(
    dir_path: &Path,
    order: Order,
) -> EResult<Box<dyn Iterator<Item = OsString> + '_>> {
    iter_snapshot_i_in_dir::<OsString>(dir_path.to_path_buf(), order, |ude| ude.file_name())
}

pub fn iter_snapshot_paths_in_dir(
    dir_path: &Path,
    order: Order,
) -> EResult<Box<dyn Iterator<Item = PathBuf> + '_>> {
    iter_snapshot_i_in_dir::<PathBuf>(dir_path.to_path_buf(), order, |ude| ude.path())
}

pub fn iter_snapshot_names_for_archive(
    archive_name: &str,
    order: Order,
) -> EResult<Box<dyn Iterator<Item = OsString> + '_>> {
    let dir_path = archive::get_archive_snapshot_dir_path(archive_name)?;
    iter_snapshot_i_in_dir::<OsString>(dir_path, order, |ude| ude.file_name())
}

pub fn iter_snapshot_paths_for_archive(
    archive_name: &str,
    order: Order,
) -> EResult<Box<dyn Iterator<Item = PathBuf> + '_>> {
    let dir_path = archive::get_archive_snapshot_dir_path(archive_name)?;
    iter_snapshot_i_in_dir::<PathBuf>(dir_path, order, |ude| ude.path())
}

pub fn get_snapshot_paths_in_dir(dir_path: &Path, order: Order) -> EResult<Vec<PathBuf>> {
    Ok(iter_snapshot_paths_in_dir(dir_path, order)?.collect::<Vec<_>>())
}

pub fn get_snapshot_paths_for_archive(archive_name: &str, order: Order) -> EResult<Vec<PathBuf>> {
    Ok(iter_snapshot_paths_for_archive(archive_name, order)?.collect::<Vec<_>>())
}

pub fn get_snapshot_names_in_dir(dir_path: &Path, order: Order) -> EResult<Vec<OsString>> {
    Ok(iter_snapshot_names_in_dir(dir_path, order)?.collect::<Vec<_>>())
}

pub fn get_snapshot_names_for_archive(archive_name: &str, order: Order) -> EResult<Vec<OsString>> {
    Ok(iter_snapshot_names_for_archive(archive_name, order)?.collect::<Vec<_>>())
}

// GUI interface functions
pub fn delete_named_snapshots(archive_name: &str, snapshot_names: &[OsString]) -> EResult<()> {
    let snapshot_dir_path = archive::get_archive_snapshot_dir_path(archive_name)?;
    for snapshot_name in snapshot_names.iter() {
        let snapshot_file_path = snapshot_dir_path.join(snapshot_name);
        delete_snapshot_file(&snapshot_file_path)?;
    }
    Ok(())
}

pub fn get_named_snapshot(
    archive_name: &str,
    snapshot_name: &OsStr,
) -> EResult<SnapshotPersistentData> {
    let snapshot_dir_path = archive::get_archive_snapshot_dir_path(archive_name)?;
    let snapshot_file_path = snapshot_dir_path.join(snapshot_name);
    SnapshotPersistentData::from_file(&snapshot_file_path)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotStats {
    pub file_stats: FileStats,
    pub sym_link_stats: SymLinkStats,
    pub creation_duration: Duration,
}

impl From<SnapshotPersistentData> for SnapshotStats {
    fn from(spd: SnapshotPersistentData) -> Self {
        Self {
            file_stats: spd.file_stats,
            sym_link_stats: spd.sym_link_stats,
            creation_duration: spd.creation_duration(),
        }
    }
}

pub fn get_snapshot_stats(archive_name: &str, snapshot_name: &OsStr) -> EResult<SnapshotStats> {
    let snapshot = get_named_snapshot(archive_name, snapshot_name)?;
    Ok(SnapshotStats::from(snapshot))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive;
    use crate::content;
    use fs2::FileExt;
    use std::env;
    use std::os::unix::fs::MetadataExt;
    use tempdir::TempDir;

    #[test]
    fn test_ssf_regex() {
        assert!(SS_FILE_NAME_RE.is_match("1027-09-14-20-20-59-1000"));
        assert!(SS_FILE_NAME_RE.is_match("1027-09-14-20-20-59+1000"));
    }

    #[test]
    fn test_write_snapshot() {
        let file = fs::OpenOptions::new()
            .write(true)
            .open("../test_lock_file")
            .unwrap_or_else(|err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err));
        if let Err(err) = file.lock_exclusive() {
            panic!("lock failed: {:?}", err);
        };
        let dir =
            TempDir::new("SS_TEST").unwrap_or_else(|err| panic!("open temp dir failed: {:?}", err));
        env::set_var("ERGIBUS_CONFIG_DIR", dir.path().join("config"));
        let data_dir = dir.path().join("data");
        let data_dir_str = match data_dir.to_str() {
            Some(data_dir_str) => data_dir_str,
            None => panic!("{:?}: line {:?}", file!(), line!()),
        };
        if let Err(err) = content::create_new_repo("test_repo", data_dir_str, "Sha1") {
            panic!("new repo: {:?}", err);
        }
        let my_file = Path::new("./src/snapshot.rs").canonicalize().unwrap();
        let cli_dir = Path::new("../ergibus").canonicalize().unwrap();
        let inclusions = vec![PathBuf::from("~/Documents"), cli_dir, my_file];
        let dir_exclusions = vec!["lost+found".to_string()];
        let file_exclusions = vec!["*.iso".to_string()];
        if let Err(err) = archive::create_new_archive(
            "test_ss",
            "test_repo",
            data_dir_str,
            &inclusions,
            &dir_exclusions,
            &file_exclusions,
        ) {
            panic!("new archive: {:?}", err);
        }
        {
            // need this to let sg finish before the temporary directory is destroyed
            let mut sg = match SnapshotGenerator::new("test_ss") {
                Ok(snapshot_generator) => snapshot_generator,
                Err(err) => panic!("new SG: {:?}", err),
            };
            println!("Generating for {:?}", "test_ss");
            assert!(sg.generate_snapshot().is_ok());
            println!(
                "Generating for {:?} took {:?}",
                "test_ss",
                sg.generation_duration()
            );
            assert!(sg.snapshot_available());
            let result = sg.write_snapshot();
            assert!(result.is_ok());
            assert!(!sg.snapshot_available());
            match result {
                Ok(ref ss_file_path) => {
                    match fs::metadata(ss_file_path) {
                        Ok(metadata) => println!("{:?}: {:?}", ss_file_path, metadata.size()),
                        Err(err) => {
                            panic!("Error getting size data: {:?}: {:?}", ss_file_path, err)
                        }
                    };
                    match SnapshotPersistentData::from_file(ss_file_path) {
                        Ok(ss) => println!(
                            "{:?}: {:?} {:?}",
                            ss.archive_name, ss.file_stats, ss.sym_link_stats
                        ),
                        Err(err) => panic!("Error reading: {:?}: {:?}", ss_file_path, err),
                    };
                }
                Err(err) => panic!("{:?}", err),
            }
        }
        if let Err(err) = dir.close() {
            panic!("remove temporary directory failed: {:?}", err)
        };
        if let Err(err) = file.unlock() {
            panic!("unlock failed: {:?}", err);
        };
    }
}
