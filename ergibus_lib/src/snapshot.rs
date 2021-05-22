// TODO: fix use of is_dir() and is_file() throughout this file

// Standard Library access
use std::collections::{btree_map, BTreeMap};
use std::convert::TryFrom;
use std::ffi::OsString;
use std::fs::{self, DirEntry, File};
use std::io;
use std::io::prelude::*;
use std::ops::AddAssign;
use std::path::{Component, Path, PathBuf};
use std::time;

// cargo.io crates access
use chrono::prelude::*;
use regex;
use serde_json;
use snap;
use walkdir::WalkDir;

// PW crate access
use pw_pathux::first_subpath_as_os_string;

// local modules access
use crate::archive::{self, get_archive_data, ArchiveData, Exclusions};
use crate::attributes::{Attributes, AttributesIfce};
use crate::content::{ContentManager, ContentMgmtKey};
use crate::path_buf_ext::RealPathBufType;
use crate::report::{ignore_report_or_crash, report_broken_link_or_crash};
use crate::{EResult, Error};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct FileData {
    file_name: OsString,
    attributes: Attributes,
    content_token: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct LinkData {
    file_name: OsString,
    attributes: Attributes,
    link_target: PathBuf,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct SnapshotDir {
    path: PathBuf,
    attributes: Attributes,
    subdirs: BTreeMap<String, SnapshotDir>,
    files: BTreeMap<String, FileData>,
    file_links: BTreeMap<String, LinkData>,
    subdir_links: BTreeMap<String, LinkData>,
}

fn get_entry_for_path(path: &Path) -> io::Result<fs::DirEntry> {
    let parent_dir_path = path
        .parent()
        .unwrap_or_else(|| panic!("Can't find parent directory"));
    let entries = fs::read_dir(&parent_dir_path)?;
    for entry_or_err in entries {
        if let Ok(entry) = entry_or_err {
            if entry.path() == path {
                return Ok(entry);
            }
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("{:?}: not found", path),
    ))
}

impl SnapshotDir {
    // Creation/destruction methods
    fn new<P: AsRef<Path>>(root_dir: P) -> io::Result<SnapshotDir> {
        let mut snapshot_dir = SnapshotDir::default();
        snapshot_dir.path = root_dir.as_ref().canonicalize()?;
        snapshot_dir.attributes = snapshot_dir.path.metadata()?.into();

        Ok(snapshot_dir)
    }

    fn release_contents(&self, content_mgr: &ContentManager) {
        for file_data in self.files.values() {
            if let Err(err) = content_mgr.release_contents(&file_data.content_token) {
                panic!("{:?}: line {:?}: token error: {:?}", file!(), line!(), err);
            };
        }
        for subdir in self.subdirs.values() {
            subdir.release_contents(content_mgr);
        }
    }

    fn find_or_add_subdir(&mut self, abs_subdir_path: &Path) -> io::Result<&mut SnapshotDir> {
        assert!(abs_subdir_path.is_absolute());
        match abs_subdir_path.strip_prefix(&self.path.clone()) {
            Ok(rel_path) => {
                let first_name = match first_subpath_as_os_string(rel_path) {
                    Some(fname) => fname,
                    None => return Ok(self),
                };
                let first_name_key = String::from(first_name.to_string_lossy());
                if !self.subdirs.contains_key(&first_name_key) {
                    let mut path_buf = PathBuf::new();
                    path_buf.push(self.path.clone());
                    path_buf.push(first_name.clone());
                    let snapshot_dir = SnapshotDir::new(&path_buf)?;
                    self.subdirs.insert(first_name_key.clone(), snapshot_dir);
                }
                match self.subdirs.get_mut(&first_name_key) {
                    Some(subdir) => subdir.find_or_add_subdir(abs_subdir_path),
                    None => panic!("{:?}: line {:?}", file!(), line!()),
                }
            }
            Err(err) => panic!("{:?}: line {:?}: {:?}", file!(), line!(), err),
        }
    }

    fn populate(
        &mut self,
        exclusions: &Exclusions,
        content_mgr: &ContentManager,
    ) -> (FileStats, SymLinkStats, u64) {
        let mut file_stats = FileStats::default();
        let mut sym_link_stats = SymLinkStats::default();
        let mut delta_repo_size: u64 = 0;
        match fs::read_dir(&self.path) {
            Ok(entries) => {
                for entry_or_err in entries {
                    match entry_or_err {
                        Ok(entry) => match entry.file_type() {
                            Ok(e_type) => {
                                if e_type.is_file() {
                                    if exclusions.is_excluded_file(&entry.path()) {
                                        continue;
                                    }
                                    let data = self.add_file(&entry, &content_mgr);
                                    file_stats += data.0;
                                    delta_repo_size += data.1;
                                } else if e_type.is_symlink() {
                                    if exclusions.is_excluded_file(&entry.path()) {
                                        continue;
                                    }
                                    sym_link_stats += self.add_symlink(&entry);
                                }
                            }
                            Err(err) => ignore_report_or_crash(&err, &self.path),
                        },
                        Err(err) => ignore_report_or_crash(&err, &self.path),
                    }
                }
            }
            Err(err) => ignore_report_or_crash(&err, &self.path),
        };
        (file_stats, sym_link_stats, delta_repo_size)
    }

    fn add_file(
        &mut self,
        dir_entry: &fs::DirEntry,
        content_mgr: &ContentManager,
    ) -> (FileStats, u64) {
        let file_name = dir_entry.file_name().as_os_str().to_os_string();
        let file_name_key = String::from(file_name.to_string_lossy());
        if self.files.contains_key(&file_name_key) {
            return (FileStats::default(), 0);
        }
        let attributes: Attributes = match dir_entry.metadata() {
            Ok(metadata) => metadata.into(),
            Err(err) => {
                ignore_report_or_crash(&err, &dir_entry.path());
                return (FileStats::default(), 0);
            }
        };
        let mut file = match File::open(&dir_entry.path()) {
            Ok(file) => file,
            Err(err) => {
                ignore_report_or_crash(&err, &dir_entry.path());
                return (FileStats::default(), 0);
            }
        };
        let (content_token, stored_size, delta_repo_size) =
            match content_mgr.store_contents(&mut file) {
                Ok((ct, ssz, drsz)) => (ct, ssz, drsz),
                Err(err) => panic!(
                    "{:?}: line {:?}: should not happen: {:?}",
                    file!(),
                    line!(),
                    err
                ),
            };
        let file_stats = FileStats {
            file_count: 1,
            byte_count: attributes.size(),
            stored_byte_count: stored_size,
        };
        self.files.insert(
            file_name_key,
            FileData {
                file_name,
                attributes,
                content_token,
            },
        );
        (file_stats, delta_repo_size)
    }

    fn add_symlink(&mut self, dir_entry: &fs::DirEntry) -> SymLinkStats {
        let file_name = dir_entry.file_name().as_os_str().to_os_string();
        let file_name_key = String::from(file_name.to_string_lossy());
        if self.file_links.contains_key(&file_name_key)
            || self.subdir_links.contains_key(&file_name_key)
        {
            return SymLinkStats::default();
        }
        let attributes: Attributes = match dir_entry.metadata() {
            Ok(metadata) => metadata.into(),
            Err(err) => {
                ignore_report_or_crash(&err, &dir_entry.path());
                return SymLinkStats::default();
            }
        };
        let link_target = match dir_entry.path().read_link() {
            Ok(lt) => lt,
            Err(err) => {
                ignore_report_or_crash(&err, &dir_entry.path());
                return SymLinkStats::default();
            }
        };
        let abs_target_path = match self.path.join(link_target.clone()).canonicalize() {
            Ok(atp) => atp,
            Err(ref err) => {
                report_broken_link_or_crash(err, &dir_entry.path(), &link_target);
                return SymLinkStats::default();
            }
        };
        if abs_target_path.is_file() {
            self.file_links.insert(
                file_name_key,
                LinkData {
                    file_name,
                    attributes,
                    link_target,
                },
            );
            return SymLinkStats {
                dir_sym_link_count: 0,
                file_sym_link_count: 1,
            };
        } else if abs_target_path.is_dir() {
            self.subdir_links.insert(
                file_name_key,
                LinkData {
                    file_name,
                    attributes,
                    link_target,
                },
            );
            return SymLinkStats {
                dir_sym_link_count: 1,
                file_sym_link_count: 0,
            };
        }
        SymLinkStats::default()
    }
}

struct SnapshotDirIter<'a> {
    values: btree_map::Values<'a, String, SnapshotDir>,
    subdir_iters: Vec<SnapshotDirIter<'a>>,
    current_subdir_iter: Box<Option<SnapshotDirIter<'a>>>,
}

impl<'a> Iterator for SnapshotDirIter<'a> {
    type Item = &'a SnapshotDir;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(item) = self.values.next() {
            return Some(item);
        } else {
            loop {
                if let Some(ref mut sub_iter) = *self.current_subdir_iter {
                    if let Some(item) = sub_iter.next() {
                        return Some(item);
                    }
                } else {
                    break;
                };
                self.current_subdir_iter = Box::new(self.subdir_iters.pop())
            }
        };
        None
    }
}

impl FileData {
    // Interrogation/extraction/restoration methods
    fn copy_contents_to(
        &self,
        to_file_path: &Path,
        c_mgr: &ContentManager,
        overwrite: bool,
    ) -> EResult<u64> {
        if to_file_path.exists() {
            if to_file_path.is_real_file() {
                let mut file = File::open(to_file_path)
                    .map_err(|err| Error::SnapshotReadIOError(err, to_file_path.to_path_buf()))?;
                let content_is_same = c_mgr.check_content_token(&mut file, &self.content_token)?;
                if content_is_same {
                    // nothing to do
                    return Ok(self.attributes.size());
                }
            }
            if !overwrite {
                let new_path = move_aside_file_path(to_file_path);
                fs::rename(to_file_path, &new_path).map_err(|err| {
                    Error::SnapshotMoveAsideFailed(to_file_path.to_path_buf(), err)
                })?;
            }
        }
        let mut file = File::create(to_file_path).unwrap();
        let bytes = c_mgr.write_contents_for_token(&self.content_token, &mut file)?;
        Ok(bytes)
    }
}

impl LinkData {
    // Interrogation/extraction/restoration methods
    fn copy_link_as<W>(
        &self,
        as_path: &Path,
        overwrite: bool,
        _op_errf: &mut Option<&mut W>,
    ) -> EResult<()>
    where
        W: std::io::Write,
    {
        if as_path.exists() {
            if as_path.is_symlink() {
                if let Ok(link_target) = as_path.read_link() {
                    if self.link_target == link_target {
                        return Ok(());
                    }
                }
            }
            if !overwrite {
                let new_path = move_aside_file_path(as_path);
                fs::rename(as_path, &new_path)
                    .map_err(|err| Error::SnapshotMoveAsideFailed(as_path.to_path_buf(), err))?;
            }
        }
        if cfg!(target_family = "unix") {
            use std::os::unix::fs::symlink;
            symlink(&self.link_target, as_path)
                .map_err(|err| Error::SnapshotMoveAsideFailed(as_path.to_path_buf(), err))?;
        } else {
            panic!("not implemented for this os")
        }
        Ok(())
    }
}

fn clear_way_for_new_dir(new_dir_path: &Path, overwrite: bool) -> EResult<()> {
    if new_dir_path.exists() && !new_dir_path.is_dir() {
        // Real dir or link to dir
        if overwrite {
            // Remove the file system object to make way for the directory
            fs::remove_file(new_dir_path)
                .map_err(|err| Error::SnapshotDeleteIOError(err, new_dir_path.to_path_buf()))?;
        } else {
            let new_path = move_aside_file_path(new_dir_path);
            fs::rename(new_dir_path, &new_path)
                .map_err(|err| Error::SnapshotMoveAsideFailed(new_dir_path.to_path_buf(), err))?;
        }
    };
    Ok(())
}

#[derive(PartialEq, Debug, Default, Copy, Clone)]
pub struct ExtractionStats {
    pub dir_count: u64,
    pub file_count: u64,
    pub bytes_count: u64,
    pub dir_sym_link_count: u64,
    pub file_sym_link_count: u64,
}

impl SnapshotDir {
    fn content_count(&self) -> usize {
        self.subdirs.len() + self.files.len() + self.file_links.len() + self.subdir_links.len()
    }

    fn base_dir_path(&self) -> &Path {
        if self.content_count() > 1 {
            self.path.as_path()
        } else {
            if let Some(sub_dir) = self.subdirs.values().next() {
                debug_assert!(self.subdirs.len() == 1);
                sub_dir.base_dir_path()
            } else {
                self.path.as_path()
            }
        }
    }

    fn base_dir(&self) -> &Self {
        if self.content_count() > 1 {
            self
        } else {
            if let Some(sub_dir) = self.subdirs.values().next() {
                debug_assert!(self.subdirs.len() == 1);
                sub_dir.base_dir()
            } else {
                self
            }
        }
    }

    pub fn subdir_names(&self) -> impl Iterator<Item = &String> {
        self.subdirs.keys()
    }

    pub fn subdir_link_names(&self) -> impl Iterator<Item = &String> {
        self.subdir_links.keys()
    }

    pub fn file_names(&self) -> impl Iterator<Item = &String> {
        self.files.keys()
    }

    pub fn file_link_names(&self) -> impl Iterator<Item = &String> {
        self.file_links.keys()
    }

    // Interrogation/extraction/restoration methods
    fn subdir_iter(&self, recursive: bool) -> SnapshotDirIter<'_> {
        let values = self.subdirs.values();
        let mut subdir_iters: Vec<SnapshotDirIter<'_>> = if recursive {
            self.subdirs.values().map(|s| s.subdir_iter(true)).collect()
        } else {
            Vec::new()
        };
        let current_subdir_iter = Box::new(subdir_iters.pop());
        SnapshotDirIter {
            values,
            subdir_iters,
            current_subdir_iter,
        }
    }

    pub fn get_subdir<P: AsRef<Path>>(&self, path_arg: P) -> EResult<&Self> {
        let subdir_path = path_arg.as_ref();
        let rel_path = if subdir_path.is_absolute() {
            subdir_path
                .strip_prefix(&self.path)
                .map_err(|_| Error::SnapshotUnknownSubdir(subdir_path.to_path_buf()))?
        } else {
            subdir_path
        };
        match first_subpath_as_os_string(rel_path) {
            None => Ok(self),
            Some(first_name) => {
                let first_name_key = String::from(first_name.to_string_lossy());
                match self.subdirs.get(&first_name_key) {
                    Some(sd) => {
                        let rel_path = rel_path
                            .strip_prefix(&first_name)
                            .map_err(|_| Error::SnapshotUnknownSubdir(subdir_path.to_path_buf()))?;
                        sd.get_subdir(&rel_path)
                    }
                    None => Err(Error::SnapshotUnknownSubdir(subdir_path.to_path_buf())),
                }
            }
        }
    }

    fn find_subdir(&self, abs_subdir_path: &Path) -> Option<&SnapshotDir> {
        assert!(abs_subdir_path.is_absolute());
        match abs_subdir_path.strip_prefix(&self.path) {
            Ok(rel_path) => {
                let first_name = match first_subpath_as_os_string(rel_path) {
                    Some(fname) => fname,
                    None => return Some(self),
                };
                let first_name_key = String::from(first_name.to_string_lossy());
                match self.subdirs.get(&first_name_key) {
                    Some(sd) => sd.find_subdir(abs_subdir_path),
                    None => None,
                }
            }
            Err(_) => None,
        }
    }

    fn find_file(&self, abs_file_path: &Path) -> Option<&FileData> {
        assert!(abs_file_path.is_absolute());
        if let Some(abs_dir_path) = abs_file_path.parent() {
            if let Some(subdir) = self.find_subdir(abs_dir_path) {
                if let Some(file_name) = abs_file_path.file_name() {
                    let file_name_key = String::from(file_name.to_string_lossy());
                    return subdir.files.get(&file_name_key);
                }
            }
        }
        None
    }

    fn copy_files_into(
        &self,
        into_dir_path: &Path,
        c_mgr: &ContentManager,
        overwrite: bool,
    ) -> EResult<(u64, u64)> {
        let mut count = 0;
        let mut bytes = 0;
        for file in self.files.values() {
            let new_path = into_dir_path.join(&file.file_name);
            bytes += file.copy_contents_to(&new_path, c_mgr, overwrite)?;
            count += 1;
        }
        Ok((count, bytes))
    }

    fn copy_dir_links_into<W>(
        &self,
        into_dir_path: &Path,
        overwrite: bool,
        op_errf: &mut Option<&mut W>,
    ) -> EResult<u64>
    where
        W: std::io::Write,
    {
        let mut count = 0;
        for subdir_link in self.subdir_links.values() {
            let new_link_path = into_dir_path.join(&subdir_link.file_name);
            subdir_link.copy_link_as(&new_link_path, overwrite, op_errf)?;
            count += 1;
        }
        Ok(count)
    }

    fn copy_file_links_into<W>(
        &self,
        into_dir_path: &Path,
        overwrite: bool,
        op_errf: &mut Option<&mut W>,
    ) -> EResult<u64>
    where
        W: std::io::Write,
    {
        let mut count = 0;
        for file_link in self.file_links.values() {
            let new_link_path = into_dir_path.join(&file_link.file_name);
            file_link.copy_link_as(&new_link_path, overwrite, op_errf)?;
            count += 1;
        }
        Ok(count)
    }

    pub fn copy_to<W>(
        &self,
        to_dir_path: &Path,
        c_mgt_key: &ContentMgmtKey,
        overwrite: bool,
        op_errf: &mut Option<&mut W>,
    ) -> EResult<ExtractionStats>
    where
        W: std::io::Write,
    {
        let mut stats = ExtractionStats::default();
        clear_way_for_new_dir(to_dir_path, overwrite)?;
        if !to_dir_path.is_dir() {
            fs::create_dir_all(to_dir_path)
                .map_err(|err| Error::SnapshotDirIOError(err, to_dir_path.to_path_buf()))?;
            if let Some(to_dir) = self.find_subdir(to_dir_path) {
                to_dir
                    .attributes
                    .set_file_attributes(to_dir_path, op_errf)
                    .map_err(|err| Error::ContentCopyIOError(err))?;
            }
        }
        stats.dir_count += 1;
        // First create all of the sub directories
        for subdir in self.subdir_iter(true) {
            let path_tail = subdir.path.strip_prefix(&self.path).unwrap(); // Should not fail
            let new_dir_path = to_dir_path.join(path_tail);
            clear_way_for_new_dir(&new_dir_path, overwrite)?;
            if !new_dir_path.is_dir() {
                fs::create_dir_all(&new_dir_path)
                    .map_err(|err| Error::SnapshotDirIOError(err, new_dir_path.to_path_buf()))?;
                subdir
                    .attributes
                    .set_file_attributes(&new_dir_path, op_errf)
                    .map_err(|err| Error::ContentCopyIOError(err))?;
            }
            stats.dir_count += 1;
        }
        // then do links to subdirs
        stats.dir_sym_link_count += self.copy_dir_links_into(&to_dir_path, overwrite, op_errf)?;
        for subdir in self.subdir_iter(true) {
            let path_tail = subdir.path.strip_prefix(&self.path).unwrap(); // Should not fail
            let new_dir_path = to_dir_path.join(path_tail);
            stats.dir_sym_link_count +=
                subdir.copy_dir_links_into(&new_dir_path, overwrite, op_errf)?;
        }
        // then do all the files (holding lock as little as needed)
        match c_mgt_key.open_content_manager(dychatat::Mutability::Immutable) {
            Ok(ref c_mgr) => {
                let (count, bytes) = self.copy_files_into(&to_dir_path, c_mgr, overwrite)?;
                stats.file_count += count;
                stats.bytes_count += bytes;
                for subdir in self.subdir_iter(true) {
                    let path_tail = subdir.path.strip_prefix(&self.path).unwrap(); // Should not fail
                    let new_dir_path = to_dir_path.join(path_tail);
                    let (count, bytes) = subdir.copy_files_into(&new_dir_path, c_mgr, overwrite)?;
                    stats.file_count += count;
                    stats.bytes_count += bytes;
                }
            }
            Err(err) => return Err(err.into()),
        }
        // then do links to file
        stats.file_sym_link_count += self.copy_file_links_into(&to_dir_path, overwrite, op_errf)?;
        for subdir in self.subdir_iter(true) {
            let path_tail = subdir.path.strip_prefix(&self.path).unwrap(); // Should not fail
            let new_dir_path = to_dir_path.join(path_tail);
            stats.file_sym_link_count +=
                subdir.copy_file_links_into(&new_dir_path, overwrite, op_errf)?;
        }
        Ok(stats)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Default, Copy, Clone)]
pub struct FileStats {
    pub file_count: u64,
    pub byte_count: u64,
    pub stored_byte_count: u64,
}

impl AddAssign for FileStats {
    fn add_assign(&mut self, other: FileStats) {
        *self = FileStats {
            file_count: self.file_count + other.file_count,
            byte_count: self.byte_count + other.byte_count,
            stored_byte_count: self.stored_byte_count + other.stored_byte_count,
        };
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Default, Copy, Clone)]
pub struct SymLinkStats {
    pub dir_sym_link_count: u64,
    pub file_sym_link_count: u64,
}

impl AddAssign for SymLinkStats {
    fn add_assign(&mut self, other: SymLinkStats) {
        *self = SymLinkStats {
            dir_sym_link_count: self.dir_sym_link_count + other.dir_sym_link_count,
            file_sym_link_count: self.file_sym_link_count + other.file_sym_link_count,
        };
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct SnapshotPersistentData {
    root_dir: SnapshotDir,
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
        let root_dir = SnapshotDir::new(Component::RootDir)?;
        Ok(Self {
            root_dir,
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

    fn release_contents(&self) {
        let content_mgr = self
            .content_mgmt_key
            .open_content_manager(dychatat::Mutability::Mutable)
            .unwrap_or_else(|err| {
                panic!(
                    "{:?}: line {:?}: open content manager: {:?}",
                    file!(),
                    line!(),
                    err
                )
            });
        self.root_dir.release_contents(&content_mgr);
    }

    fn add_dir(&mut self, abs_dir_path: &Path, exclusions: &Exclusions) -> io::Result<u64> {
        let dir = self.root_dir.find_or_add_subdir(&abs_dir_path)?;
        let content_mgr = self
            .content_mgmt_key
            .open_content_manager(dychatat::Mutability::Mutable)
            .unwrap_or_else(|err| {
                panic!(
                    "{:?}: line {:?}: open content manager: {:?}",
                    file!(),
                    line!(),
                    err
                )
            });
        let (file_stats, sym_link_stats, drsz) = dir.populate(exclusions, &content_mgr);
        self.file_stats += file_stats;
        self.sym_link_stats += sym_link_stats;
        let mut delta_repo_size = drsz;
        for entry in WalkDir::new(abs_dir_path)
            .into_iter()
            .filter_entry(|e| exclusions.is_non_excluded_dir(e))
        {
            match entry {
                Ok(e_data) => {
                    let e_path = e_data.path();
                    match dir.find_or_add_subdir(e_path) {
                        Ok(sub_dir) => {
                            let (file_stats, sym_link_stats, drsz) =
                                sub_dir.populate(exclusions, &content_mgr);
                            self.file_stats += file_stats;
                            self.sym_link_stats += sym_link_stats;
                            delta_repo_size += drsz;
                        }
                        Err(err) => ignore_report_or_crash(&err, &e_path),
                    }
                }
                Err(err) => {
                    let path_buf = match err.path() {
                        Some(path) => path.to_path_buf(),
                        None => panic!("{:?}: line {:?}", file!(), line!()),
                    };
                    let io_error = io::Error::from(err);
                    ignore_report_or_crash(&io_error, &path_buf);
                }
            }
        }
        Ok(delta_repo_size)
    }

    fn add_other(&mut self, abs_file_path: &Path) -> io::Result<u64> {
        let entry = get_entry_for_path(abs_file_path)?;
        let dir_path = abs_file_path
            .parent()
            .unwrap_or_else(|| panic!("{:?}: line {:?}", file!(), line!()));
        let dir = self.root_dir.find_or_add_subdir(&dir_path)?;
        let mut delta_repo_size: u64 = 0;
        match entry.file_type() {
            Ok(e_type) => {
                if e_type.is_file() {
                    let content_mgr = self
                        .content_mgmt_key
                        .open_content_manager(dychatat::Mutability::Mutable)
                        .unwrap_or_else(|err| {
                            panic!(
                                "{:?}: line {:?}: open content manager: {:?}",
                                file!(),
                                line!(),
                                err
                            )
                        });
                    let data = dir.add_file(&entry, &content_mgr);
                    self.file_stats += data.0;
                    delta_repo_size += data.1;
                } else if e_type.is_symlink() {
                    self.sym_link_stats += dir.add_symlink(&entry);
                }
            }
            Err(err) => ignore_report_or_crash(&err, abs_file_path),
        };
        Ok(delta_repo_size)
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

    fn write_to_dir(&self, dir_path: &Path) -> EResult<PathBuf> {
        let file_name = self.snapshot_name();
        let path = dir_path.join(file_name);
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

fn entry_is_ss_file(entry: &DirEntry) -> bool {
    let path = entry.path();
    if path.is_file() {
        if let Some(file_name) = path.file_name() {
            if let Some(file_name) = file_name.to_str() {
                return SS_FILE_NAME_RE.is_match(file_name);
            }
        }
    }
    false
}

fn get_ss_entries_in_dir(dir_path: &Path) -> EResult<Vec<DirEntry>> {
    let dir_entries = fs::read_dir(dir_path)
        .map_err(|err| Error::SnapshotDirIOError(err, dir_path.to_path_buf()))?;
    let mut ss_entries = Vec::new();
    for entry_or_err in dir_entries {
        match entry_or_err {
            Ok(entry) => {
                if entry_is_ss_file(&entry) {
                    ss_entries.push(entry);
                }
            }
            Err(_) => (),
        }
    }
    ss_entries.sort_by_key(|e| e.path());
    Ok(ss_entries)
}

fn move_aside_file_path(path: &Path) -> PathBuf {
    let dt = DateTime::<Local>::from(time::SystemTime::now());
    let suffix = format!("{}", dt.format("ema-%Y-%m-%d-%H-%M-%S"));
    let new_suffix = if let Some(current_suffix) = path.extension() {
        format!("{:?}-{}", current_suffix, suffix)
    } else {
        suffix
    };
    path.with_extension(&new_suffix)
}

impl SnapshotPersistentData {
    // Interrogation/extraction/restoration methods

    pub fn base_dir_path(&self) -> &Path {
        self.root_dir.base_dir_path()
    }

    pub fn base_dir(&self) -> &SnapshotDir {
        self.root_dir.base_dir()
    }

    pub fn from_file(file_path: &Path) -> EResult<SnapshotPersistentData> {
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

    fn archive_name(&self) -> &str {
        &self.archive_name
    }

    pub fn copy_file_to(
        &self,
        fm_file_path: &Path,
        to_file_path: &Path,
        overwrite: bool,
    ) -> EResult<u64> {
        let file_data = match self.root_dir.find_file(fm_file_path) {
            Some(fd) => fd,
            None => {
                return Err(Error::SnapshotUnknownFile(
                    self.archive_name().to_string(),
                    self.snapshot_name(),
                    fm_file_path.to_path_buf(),
                ))
            }
        };
        let c_mgr = self
            .content_mgmt_key
            .open_content_manager(dychatat::Mutability::Immutable)?;
        let bytes = file_data.copy_contents_to(to_file_path, &c_mgr, overwrite)?;
        Ok(bytes)
    }

    pub fn copy_dir_to<W>(
        &self,
        fm_dir_path: &Path,
        to_dir_path: &Path,
        overwrite: bool,
        op_errf: &mut Option<&mut W>,
    ) -> EResult<ExtractionStats>
    where
        W: std::io::Write,
    {
        let fm_subdir = if let Some(subdir) = self.root_dir.find_subdir(fm_dir_path) {
            subdir
        } else {
            return Err(Error::SnapshotUnknownDirectory(
                self.archive_name().to_string(),
                self.snapshot_name(),
                fm_dir_path.to_path_buf(),
            ));
        };
        let stats = fm_subdir.copy_to(to_dir_path, &self.content_mgmt_key, overwrite, op_errf)?;
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
            self.release_snapshot();
        }
    }
}

impl SnapshotGenerator {
    pub fn new(archive_name: &str) -> EResult<SnapshotGenerator> {
        let archive_data = get_archive_data(archive_name)?;
        // Check that there'll be no problem creating snapshots
        let _dummy = SnapshotPersistentData::try_from(&archive_data)?;
        let snapshot: Option<SnapshotPersistentData> = None;
        Ok(SnapshotGenerator {
            snapshot,
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
            self.release_snapshot();
        }
        let mut delta_repo_size: u64 = 0;
        let mut snapshot = SnapshotPersistentData::try_from(&self.archive_data)?;
        for abs_path in self.archive_data.includes.iter() {
            if abs_path.is_dir() {
                match snapshot.add_dir(&abs_path, &self.archive_data.exclusions) {
                    Ok(drsz) => delta_repo_size += drsz,
                    Err(err) => ignore_report_or_crash(&err, &abs_path),
                };
            } else {
                match snapshot.add_other(&abs_path) {
                    Ok(drsz) => delta_repo_size += drsz,
                    Err(err) => ignore_report_or_crash(&err, &abs_path),
                };
            }
        }
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

    fn release_snapshot(&mut self) {
        match self.snapshot {
            Some(ref snapshot) => snapshot.release_contents(),
            None => (),
        }
        self.snapshot = None;
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
    snapshot.release_contents();
    Ok(())
}

pub fn get_snapshot_paths_in_dir(dir_path: &Path, reverse: bool) -> EResult<Vec<PathBuf>> {
    let entries = get_ss_entries_in_dir(dir_path)?;
    let mut snapshot_paths = Vec::new();
    for entry in entries {
        let e_path = dir_path.join(entry.path());
        snapshot_paths.push(e_path);
    }
    if reverse {
        snapshot_paths.reverse();
    };
    Ok(snapshot_paths)
}

pub fn get_snapshot_paths_for_archive(archive_name: &str, reverse: bool) -> EResult<Vec<PathBuf>> {
    let snapshot_dir_path = archive::get_archive_snapshot_dir_path(archive_name)?;
    let snapshot_paths = get_snapshot_paths_in_dir(&snapshot_dir_path, reverse)?;
    Ok(snapshot_paths)
}

pub fn get_snapshot_names_in_dir(dir_path: &Path, reverse: bool) -> EResult<Vec<String>> {
    let entries = get_ss_entries_in_dir(dir_path)?;
    let mut snapshot_names = Vec::new();
    for entry in entries {
        snapshot_names.push(String::from(entry.file_name().to_string_lossy().to_owned()));
    }
    if reverse {
        snapshot_names.reverse();
    };
    Ok(snapshot_names)
}

pub fn get_snapshot_names_for_archive(archive_name: &str, reverse: bool) -> EResult<Vec<String>> {
    let snapshot_dir_path = archive::get_archive_snapshot_dir_path(archive_name)?;
    let snapshot_names = get_snapshot_names_in_dir(&snapshot_dir_path, reverse)?;
    Ok(snapshot_names)
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
    fn find_or_add_subdir_works() {
        let mut sd = SnapshotDir::new(None)
            .unwrap_or_else(|err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err));
        let p = PathBuf::from("../TEST").canonicalize().unwrap();
        {
            let ssd = sd.find_or_add_subdir(&p);
            assert!(ssd.is_ok());
            let ssd =
                ssd.unwrap_or_else(|err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err));
            assert!(ssd.path == p.as_path());
        }
        let ssd = match sd.find_subdir(&p) {
            Some(ssd) => ssd,
            None => panic!("{:?}: line {:?}", file!(), line!()),
        };
        assert!(ssd.path == p.as_path());
        let sdp = PathBuf::from("../").canonicalize().unwrap();
        let ssd = match sd.find_subdir(&sdp) {
            Some(ssd) => ssd,
            None => panic!("{:?}: line {:?}", file!(), line!()),
        };
        assert_eq!(ssd.path, sdp.as_path());
        let sdp1 = PathBuf::from("../TEST/config").canonicalize().unwrap();
        assert_eq!(sd.find_subdir(&sdp1), None);
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
        let my_file = Path::new("./src/snapshot.rs")
            .canonicalize()
            .unwrap_or_else(|err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err));
        let my_file = my_file
            .to_str()
            .unwrap_or_else(|| panic!("{:?}: line {:?}", file!(), line!()));
        let cli_dir = Path::new("../ergibus")
            .canonicalize()
            .unwrap_or_else(|err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err));
        let cli_dir = cli_dir
            .to_str()
            .unwrap_or_else(|| panic!("{:?}: line {:?}", file!(), line!()));
        let inclusions = vec![
            "~/Documents".to_string(),
            cli_dir.to_string(),
            my_file.to_string(),
        ];
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
            sg.generate_snapshot();
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
