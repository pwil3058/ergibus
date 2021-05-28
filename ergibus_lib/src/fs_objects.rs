// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::archive::Exclusions;
use crate::attributes::{Attributes, AttributesIfce};
use crate::content::{ContentManager, ContentMgmtKey};
use crate::path_buf_ext::RealPathBufType;
use crate::report::ignore_report_or_fail;
use crate::{EResult, Error, UNEXPECTED};
use chrono::{DateTime, Local};
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::{self, File};
use std::io::ErrorKind;
use std::ops::AddAssign;
use std::path::{Component, Path, PathBuf};
use std::time;

pub trait Name {
    fn name(&self) -> &OsStr;
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
pub struct FileData {
    file_name: OsString,
    attributes: Attributes,
    content_token: String,
}

impl Name for FileData {
    fn name(&self) -> &OsStr {
        &self.file_name
    }
}

impl FileData {
    pub fn file_system_object<P: AsRef<Path>>(
        path_arg: P,
        content_manager: &ContentManager,
    ) -> EResult<(FileSystemObject, FileStats, u64)> {
        let path = path_arg.as_ref();
        let attributes: Attributes = path.metadata()?.into();
        let mut file = File::open(path)?;
        let (content_token, stored_size, delta_repo_size) =
            content_manager.store_contents(&mut file)?;
        let file_stats = FileStats {
            file_count: 1,
            byte_count: attributes.size(),
            stored_byte_count: stored_size,
        };
        let file_name = path_arg
            .as_ref()
            .file_name()
            .expect(UNEXPECTED)
            .to_os_string();
        let file_data = Self {
            file_name,
            attributes,
            content_token,
        };
        Ok((
            FileSystemObject::File(file_data),
            file_stats,
            delta_repo_size,
        ))
    }

    // Interrogation/extraction/restoration methods
    pub fn copy_contents_to(
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

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
pub struct SymLinkData {
    file_name: OsString,
    attributes: Attributes,
    link_target: PathBuf,
}

impl Name for SymLinkData {
    fn name(&self) -> &OsStr {
        &self.file_name
    }
}

impl SymLinkData {
    pub fn file_system_object<P: AsRef<Path>>(
        path_arg: P,
    ) -> EResult<(FileSystemObject, SymLinkStats)> {
        let path = path_arg.as_ref();
        let attributes: Attributes = path.symlink_metadata()?.into();
        let is_file = path.metadata()?.is_file();
        let file_name = path_arg
            .as_ref()
            .file_name()
            .expect(UNEXPECTED)
            .to_os_string();
        let link_target = path.read_link()?;
        match path
            .parent()
            .unwrap()
            .join(link_target.clone())
            .canonicalize()
        {
            Ok(_) => (),
            Err(err) => match err.kind() {
                ErrorKind::NotFound => {
                    return Err(Error::FSOBrokenSymLink(path.to_path_buf(), link_target))
                }
                _ => return Err(err.into()),
            },
        }
        let sym_link_data = Self {
            file_name,
            attributes,
            link_target,
        };
        let sym_link_stats = if is_file {
            SymLinkStats {
                dir_sym_link_count: 0,
                file_sym_link_count: 1,
            }
        } else {
            SymLinkStats {
                dir_sym_link_count: 0,
                file_sym_link_count: 1,
            }
        };
        Ok((
            FileSystemObject::SymLink(sym_link_data, is_file),
            sym_link_stats,
        ))
    }
}

impl SymLinkData {
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

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
pub struct DirectoryData {
    pub(crate) path: PathBuf,
    attributes: Attributes,
    pub(crate) contents: Vec<FileSystemObject>,
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

impl DirectoryData {
    pub fn try_new<P: AsRef<Path>>(root_dir: P) -> EResult<Self> {
        let mut dir_data = Self::default();
        dir_data.path = root_dir.as_ref().canonicalize()?;
        dir_data.attributes = dir_data.path.metadata()?.into();

        Ok(dir_data)
    }

    pub fn file_system_object<P: AsRef<Path>>(root_dir: P) -> EResult<FileSystemObject> {
        Ok(FileSystemObject::Directory(Self::try_new(root_dir)?))
    }

    #[inline]
    pub fn index_for(&self, name: &OsStr) -> Result<usize, usize> {
        self.contents.binary_search_by_key(&name, |o| o.name())
    }

    pub fn files(&self) -> impl Iterator<Item = &FileData> {
        self.contents.iter().filter_map(|o| o.get_file_data())
    }

    pub fn subdirs(&self) -> impl Iterator<Item = &DirectoryData> {
        self.contents.iter().filter_map(|o| o.get_dir_data())
    }

    pub fn release_contents(&self, content_mgr: &ContentManager) -> EResult<()> {
        for file_data in self.files() {
            content_mgr.release_contents(&file_data.content_token)?;
        }
        for subdir in self.subdirs() {
            subdir.release_contents(content_mgr)?;
        }
        Ok(())
    }

    pub fn find_or_add_subdir<P>(&mut self, path_arg: P) -> EResult<&mut DirectoryData>
    where
        P: AsRef<Path>,
    {
        let abs_subdir_path = path_arg.as_ref();
        debug_assert!(abs_subdir_path.is_absolute());
        let rel_path = abs_subdir_path.strip_prefix(&self.path).expect(UNEXPECTED);
        match rel_path.components().next() {
            None => Ok(self),
            Some(Component::Normal(first_name)) => match self.index_for(first_name) {
                Ok(index) => self.contents[index]
                    .get_dir_data_mut()
                    .expect(UNEXPECTED)
                    .find_or_add_subdir(abs_subdir_path),
                Err(index) => {
                    let file_system_object =
                        DirectoryData::file_system_object(&self.path.join(first_name))?;
                    self.contents.insert(index, file_system_object);
                    self.contents[index]
                        .get_dir_data_mut()
                        .expect(UNEXPECTED)
                        .find_or_add_subdir(abs_subdir_path)
                }
            },
            _ => Err(Error::FSOMalformedPath(rel_path.to_path_buf())),
        }
    }

    pub fn populate(
        &mut self,
        exclusions: &Exclusions,
        content_mgr: &ContentManager,
    ) -> EResult<(FileStats, SymLinkStats, u64)> {
        let mut file_stats = FileStats::default();
        let mut sym_link_stats = SymLinkStats::default();
        let mut delta_repo_size: u64 = 0;
        match fs::read_dir(&self.path) {
            Ok(read_dir) => {
                // TODO: use size_hint() to reserve sufficient space in contents vector
                for entry in read_dir.filter_map(|e| e.ok()) {
                    if exclusions.is_excluded(&entry)? {
                        continue;
                    }
                    let name = entry.file_name();
                    match self.index_for(&name) {
                        Ok(index) => match self.contents[index].get_dir_data_mut() {
                            Some(dir_data) => match dir_data.populate(exclusions, content_mgr) {
                                Ok(stats) => {
                                    file_stats += stats.0;
                                    sym_link_stats += stats.1;
                                    delta_repo_size += stats.2;
                                }
                                Err(err) => ignore_report_or_fail(err, &self.path)?,
                            },
                            _ => (),
                        },
                        Err(index) => match entry.file_type() {
                            Ok(e_type) => {
                                let path = entry.path();
                                if e_type.is_dir() {
                                    match DirectoryData::file_system_object(&path) {
                                        Ok(mut file_system_object) => {
                                            match file_system_object
                                                .get_dir_data_mut()
                                                .expect(UNEXPECTED)
                                                .populate(exclusions, content_mgr)
                                            {
                                                Ok(stats) => {
                                                    file_stats += stats.0;
                                                    sym_link_stats += stats.1;
                                                    delta_repo_size += stats.2;
                                                    self.contents.insert(index, file_system_object);
                                                }
                                                Err(err) => ignore_report_or_fail(err, &path)?,
                                            }
                                        }
                                        Err(err) => ignore_report_or_fail(err, &path)?,
                                    }
                                } else if e_type.is_file() {
                                    match FileData::file_system_object(&path, content_mgr) {
                                        Ok((file_system_object, stats, delta)) => {
                                            file_stats += stats;
                                            delta_repo_size += delta;
                                            self.contents.insert(index, file_system_object);
                                        }
                                        Err(err) => ignore_report_or_fail(err, &path)?,
                                    }
                                } else if e_type.is_symlink() {
                                    match SymLinkData::file_system_object(&path) {
                                        Ok((file_system_object, stats)) => {
                                            sym_link_stats += stats;
                                            self.contents.insert(index, file_system_object);
                                        }
                                        Err(err) => ignore_report_or_fail(err, &path)?,
                                    }
                                }
                            }
                            Err(err) => ignore_report_or_fail(err.into(), &entry.path())?,
                        },
                    }
                }
            }
            Err(err) => ignore_report_or_fail(err.into(), &self.path)?,
        };
        Ok((file_stats, sym_link_stats, delta_repo_size))
    }
}

impl Name for DirectoryData {
    fn name(&self) -> &OsStr {
        self.path.file_name().expect(UNEXPECTED)
    }
}

struct SubdirIter<'a> {
    contents: &'a Vec<FileSystemObject>,
    index: usize,
    subdir_iters: Vec<SubdirIter<'a>>,
    current_subdir_iter: Box<Option<SubdirIter<'a>>>,
}

impl<'a> Iterator for SubdirIter<'a> {
    type Item = &'a DirectoryData;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.contents.len() {
            if let Some(dir_data) = self.contents[self.index].get_dir_data() {
                self.index += 1;
                return Some(dir_data);
            } else {
                self.index += 1;
            }
        }
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
        None
    }
}

#[derive(PartialEq, Debug, Default, Copy, Clone)]
pub struct ExtractionStats {
    pub dir_count: u64,
    pub file_count: u64,
    pub bytes_count: u64,
    pub dir_sym_link_count: u64,
    pub file_sym_link_count: u64,
}

impl DirectoryData {
    // Interrogation/extraction/restoration methods
    pub fn contents(&self) -> impl Iterator<Item = &FileSystemObject> {
        self.contents.iter()
    }

    pub fn dir_sym_links(&self) -> impl Iterator<Item = &SymLinkData> {
        self.contents
            .iter()
            .filter_map(|o| o.get_dir_sym_link_data())
    }

    pub fn file_sym_links(&self) -> impl Iterator<Item = &SymLinkData> {
        self.contents
            .iter()
            .filter_map(|o| o.get_file_sym_link_data())
    }

    pub fn get_directory(&self, name: &OsStr) -> Option<&DirectoryData> {
        match self.index_for(name) {
            Ok(index) => self.contents[index].get_dir_data(),
            Err(_) => None,
        }
    }

    pub fn get_file(&self, name: &OsStr) -> Option<&FileData> {
        match self.index_for(name) {
            Ok(index) => self.contents[index].get_file_data(),
            Err(_) => None,
        }
    }

    fn subdir_iter<'a>(&'a self, recursive: bool) -> SubdirIter<'a> {
        let contents = &self.contents;
        let mut subdir_iters: Vec<SubdirIter<'a>> = if recursive {
            self.subdirs().map(|s| s.subdir_iter(true)).collect()
        } else {
            Vec::new()
        };
        let current_subdir_iter = Box::new(subdir_iters.pop());
        SubdirIter {
            contents,
            index: 0,
            subdir_iters,
            current_subdir_iter,
        }
    }

    pub fn find_subdir<P: AsRef<Path>>(&self, path_arg: P) -> EResult<&Self> {
        let subdir_path = path_arg.as_ref();
        debug_assert!(subdir_path.is_absolute());
        let rel_path = subdir_path
            .strip_prefix(&self.path)
            .map_err(|_| Error::SnapshotUnknownDirectory(subdir_path.to_path_buf()))?;
        match rel_path.components().next() {
            None => Ok(self),
            Some(Component::Normal(first_name)) => match self.get_directory(&first_name) {
                Some(sd) => sd.find_subdir(path_arg),
                None => Err(Error::SnapshotUnknownDirectory(subdir_path.to_path_buf())),
            },
            _ => Err(Error::FSOMalformedPath(rel_path.to_path_buf())),
        }
    }

    pub fn find_file<P: AsRef<Path>>(&self, file_path_arg: P) -> EResult<&FileData> {
        let file_path = file_path_arg.as_ref();
        match file_path.file_name() {
            Some(file_name) => {
                if let Some(dir_path) = file_path.parent() {
                    let subdir = self.find_subdir(dir_path)?;
                    match subdir.get_file(file_name) {
                        Some(file_data) => Ok(file_data),
                        None => Err(Error::SnapshotUnknownFile(file_path.to_path_buf())),
                    }
                } else {
                    match self.get_file(file_name) {
                        Some(file_data) => Ok(file_data),
                        None => Err(Error::SnapshotUnknownFile(file_path.to_path_buf())),
                    }
                }
            }
            None => Err(Error::SnapshotUnknownFile(file_path.to_path_buf())),
        }
    }

    fn copy_files_into(
        &self,
        into_dir_path: &Path,
        c_mgr: &ContentManager,
        overwrite: bool,
    ) -> EResult<(u64, u64)> {
        let mut count = 0;
        let mut bytes = 0;
        for file in self.files() {
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
        for subdir_link in self.dir_sym_links() {
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
        for file_link in self.file_sym_links() {
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
        // TODO: Add hard link retention to copying of directories
        let mut stats = ExtractionStats::default();
        clear_way_for_new_dir(to_dir_path, overwrite)?;
        if !to_dir_path.is_dir() {
            fs::create_dir_all(to_dir_path)
                .map_err(|err| Error::SnapshotDirIOError(err, to_dir_path.to_path_buf()))?;
            if let Ok(to_dir) = self.find_subdir(to_dir_path) {
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

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum FileSystemObject {
    File(FileData),
    SymLink(SymLinkData, bool),
    Directory(DirectoryData),
}

impl Name for FileSystemObject {
    fn name(&self) -> &OsStr {
        use FileSystemObject::*;
        match self {
            File(file_data) => file_data.name(),
            SymLink(link_data, _) => link_data.name(),
            Directory(dir_data) => dir_data.name(),
        }
    }
}

impl fmt::Display for FileSystemObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use FileSystemObject::*;
        match self {
            File(file_data) => write!(f, "{}", file_data.name().to_string_lossy()),
            Directory(dir_data) => write!(f, "{}/", dir_data.name().to_string_lossy()),
            SymLink(link_data, _) => write!(
                f,
                "{} -> {}",
                link_data.name().to_string_lossy(),
                link_data.link_target.to_string_lossy()
            ),
        }
    }
}

impl FileSystemObject {
    pub fn get_dir_data(&self) -> Option<&DirectoryData> {
        use FileSystemObject::*;
        match self {
            Directory(dir_data) => Some(dir_data),
            _ => None,
        }
    }

    pub fn get_dir_data_mut(&mut self) -> Option<&mut DirectoryData> {
        use FileSystemObject::*;
        match self {
            Directory(dir_data) => Some(dir_data),
            _ => None,
        }
    }

    pub fn get_file_data(&self) -> Option<&FileData> {
        use FileSystemObject::*;
        match self {
            File(file_data) => Some(file_data),
            _ => None,
        }
    }

    pub fn get_file_sym_link_data(&self) -> Option<&SymLinkData> {
        use FileSystemObject::*;
        match self {
            SymLink(link_data, true) => Some(link_data),
            _ => None,
        }
    }

    pub fn get_dir_sym_link_data(&self) -> Option<&SymLinkData> {
        use FileSystemObject::*;
        match self {
            SymLink(link_data, false) => Some(link_data),
            _ => None,
        }
    }
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
