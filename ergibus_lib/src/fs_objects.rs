// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::archive::Exclusions;
use crate::attributes::Attributes;
use crate::content::ContentManager;
use crate::report::{ignore_report_or_crash, ignore_report_or_fail};
use crate::{EResult, Error, UNEXPECTED};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::fs::File;
use std::ops::AddAssign;
use std::path::{Component, Path, PathBuf};

pub trait Key {
    fn key(&self) -> &OsStr;
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
pub struct FileData {
    file_name: OsString,
    attributes: Attributes,
    content_token: String,
}

impl Key for FileData {
    fn key(&self) -> &OsStr {
        &self.file_name
    }
}

impl FileData {
    pub fn try_new<P: AsRef<Path>>(
        path_arg: P,
        content_manager: &ContentManager,
    ) -> EResult<(Self, u64, u64)> {
        let path = path_arg.as_ref();
        let attributes: Attributes = path.metadata()?.into();
        let mut file = File::open(path)?;
        let (content_token, stored_size, delta_repo_size) =
            content_manager.store_contents(&mut file)?;
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
        Ok((file_data, stored_size, delta_repo_size))
    }
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
pub struct SymLinkData {
    file_name: OsString,
    attributes: Attributes,
    link_target: PathBuf,
}

impl Key for SymLinkData {
    fn key(&self) -> &OsStr {
        &self.file_name
    }
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
pub struct DirectoryData {
    pub(crate) path: PathBuf,
    attributes: Attributes,
    file_system_objects: FileSystemObjects,
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

    pub fn release_contents(&self, content_mgr: &ContentManager) -> EResult<()> {
        for file_data in self.file_system_objects.files() {
            content_mgr.release_contents(&file_data.content_token)?;
        }
        for subdir in self.file_system_objects.subdirs() {
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
            Some(Component::Normal(first_name)) => {
                let subdir = self
                    .file_system_objects
                    .get_or_insert_dir(first_name, &self.path)?;
                subdir.find_or_add_subdir(abs_subdir_path)
            }
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
            Ok(entries) => {
                for entry_or_err in entries {
                    match entry_or_err {
                        Ok(entry) => match entry.file_type() {
                            Ok(e_type) => {
                                // if e_type.is_file() {
                                //     if exclusions.is_excluded_file(&entry.path()) {
                                //         continue;
                                //     }
                                //     let data = self.add_file(&entry, &content_mgr);
                                //     file_stats += data.0;
                                //     delta_repo_size += data.1;
                                // } else if e_type.is_symlink() {
                                //     if exclusions.is_excluded_file(&entry.path()) {
                                //         continue;
                                //     }
                                //     sym_link_stats += self.add_symlink(&entry);
                                // }
                            }
                            Err(err) => ignore_report_or_fail(err.into(), &self.path)?,
                        },
                        Err(err) => ignore_report_or_fail(err.into(), &self.path)?,
                    }
                }
            }
            Err(err) => ignore_report_or_fail(err.into(), &self.path)?,
        };
        Ok((file_stats, sym_link_stats, delta_repo_size))
    }
}

impl Key for DirectoryData {
    fn key(&self) -> &OsStr {
        self.path.file_name().expect(UNEXPECTED)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum FileSystemObject {
    File(FileData),
    SymLink(SymLinkData, bool),
    Directory(DirectoryData),
}

impl Key for FileSystemObject {
    fn key(&self) -> &OsStr {
        use FileSystemObject::*;
        match self {
            File(file_data) => file_data.key(),
            SymLink(link_data, _) => link_data.key(),
            Directory(dir_data) => dir_data.key(),
        }
    }
}

impl FileSystemObject {
    pub fn new(entry: &fs::DirEntry, content_manager: &ContentManager) -> EResult<Self> {
        let path_buf = entry.path();
        let file_type = entry.file_type()?;
        //if file_type.is_dir() {
        let dir_data = DirectoryData::try_new(&path_buf)?;
        Ok(FileSystemObject::Directory(dir_data))
        //}
    }

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

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
pub struct FileSystemObjects(Vec<FileSystemObject>);

impl FileSystemObjects {
    #[inline]
    fn key_index(&self, key: &OsStr) -> Result<usize, usize> {
        self.0.binary_search_by_key(&key, |o| o.key())
    }

    pub fn insert(&mut self, fs_obj: FileSystemObject) -> EResult<()> {
        match self.key_index(fs_obj.key()) {
            Ok(_) => Err(Error::DuplicateFileSystemObjectName),
            Err(index) => {
                self.0.insert(index, fs_obj);
                Ok(())
            }
        }
    }

    pub fn get_or_insert_dir<P>(&mut self, key: &OsStr, parent: P) -> EResult<&mut DirectoryData>
    where
        P: AsRef<Path>,
    {
        match self.key_index(key) {
            Ok(index) => match self.0[index].get_dir_data_mut() {
                Some(file_data) => Ok(file_data),
                None => Err(Error::DuplicateFileSystemObjectName),
            },
            Err(index) => {
                let dir_data = DirectoryData::try_new(&parent.as_ref().join(key))?;
                self.0.insert(index, FileSystemObject::Directory(dir_data));
                Ok(self.0[index].get_dir_data_mut().expect(UNEXPECTED))
            }
        }
    }

    pub fn get(&self, key: &OsStr) -> Option<&FileSystemObject> {
        match self.key_index(key) {
            Ok(index) => Some(&self.0[index]),
            Err(_) => None,
        }
    }

    pub fn get_directory(&self, key: &OsStr) -> Option<&DirectoryData> {
        match self.key_index(key) {
            Ok(index) => self.0[index].get_dir_data(),
            Err(_) => None,
        }
    }

    pub fn files(&self) -> impl Iterator<Item = &FileData> {
        self.0.iter().filter_map(|o| o.get_file_data())
    }

    pub fn file_sym_links(&self) -> impl Iterator<Item = &SymLinkData> {
        self.0.iter().filter_map(|o| o.get_file_sym_link_data())
    }

    pub fn dir_sym_links(&self) -> impl Iterator<Item = &SymLinkData> {
        self.0.iter().filter_map(|o| o.get_dir_sym_link_data())
    }

    pub fn subdirs(&self) -> impl Iterator<Item = &DirectoryData> {
        self.0.iter().filter_map(|o| o.get_dir_data())
    }
}
