// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::attributes::Attributes;
use crate::{EResult, Error, UNEXPECTED};
use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Component, Path, PathBuf};

pub trait Key {
    fn key(&self) -> &OsStr;
}

#[derive(Serialize, Deserialize, Debug, Default)]
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

#[derive(Serialize, Deserialize, Debug, Default)]
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

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct DirectoryData {
    path: PathBuf,
    attributes: Attributes,
    file_system_objects: FileSystemObjects,
}

impl DirectoryData {
    fn new<P: AsRef<Path>>(root_dir: P) -> io::Result<Self> {
        let mut dir_data = Self::default();
        dir_data.path = root_dir.as_ref().canonicalize()?;
        dir_data.attributes = dir_data.path.metadata()?.into();

        Ok(dir_data)
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
}

impl Key for DirectoryData {
    fn key(&self) -> &OsStr {
        self.path.file_name().expect(UNEXPECTED)
    }
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug, Default)]
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
                let dir_data = DirectoryData::new(&parent.as_ref().join(key))?;
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
}
