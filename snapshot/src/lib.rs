#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

use std::io;
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf, Component};
use std::collections::HashMap;
use std::os::linux::fs::MetadataExt;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Attributes {
    st_mode: u32,
}

impl Attributes {
    pub fn new(metadata: &Metadata) -> Attributes {
        Attributes{
            st_mode: metadata.st_mode(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct SnapshotFile {
    path: PathBuf,
    attributes: Attributes,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct SnapshotSymLink {
    path: PathBuf,
    attributes: Attributes,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct SnapshotDir {
    path: PathBuf,
    attributes: Attributes,
    subdirs: HashMap<String, SnapshotDir>,
    files: HashMap<String, SnapshotFile>,
    file_links: HashMap<String, SnapshotSymLink>,
    subdir_links: HashMap<String, SnapshotSymLink>,
}

pub fn first_component_name(path: &Path) -> &str {
    assert!(path.is_relative());
    match path.components().next() {
        Some(c) => {
            match c {
                Component::Normal(c) => {
                    match c.to_str() {
                        Some(s) => s,
                        None => panic!("shouldn't happen!!!"),
                    }
                },
                _ => panic!("shouldn't happen!!!"),
            }
        },
        _ => panic!("shouldn't happen!!!"),
    }
}

impl SnapshotDir {
    pub fn new(rootdir: &Path) -> io::Result<SnapshotDir> {
        let metadata = rootdir.metadata()?;
        let path = rootdir.canonicalize()?;

        let mut subdirs = HashMap::<String, SnapshotDir>::new();
        let mut files = HashMap::<String, SnapshotFile>::new();
        let mut file_links = HashMap::<String, SnapshotSymLink>::new();
        let mut subdir_links = HashMap::<String, SnapshotSymLink>::new();

        for entry in fs::read_dir(path.as_path())? {
            let entry = entry?;
            let epathbuf = entry.path();
            let epath = epathbuf.as_path();
            match epath.file_name() {
                Some(file_name) => {
                    let emetadata = epath.metadata()?;
                    if emetadata.is_dir() {
                        if emetadata.file_type().is_symlink() {
                            let snapshot_sym_link = SnapshotSymLink{
                                path: epath.to_path_buf(),
                                attributes: Attributes::new(&emetadata)
                            };
                            subdir_links.insert(file_name.to_str().unwrap().to_string(), snapshot_sym_link);
                        } else {
                            let snapshot_dir = match SnapshotDir::new(epath) {
                                Ok(ssd) => ssd,
                                Err(_) => continue,
                            };
                            subdirs.insert(file_name.to_str().unwrap().to_string(), snapshot_dir);
                        }
                    } else {
                        if emetadata.file_type().is_symlink() {
                            let snapshot_sym_link = SnapshotSymLink{
                                path: epath.to_path_buf(),
                                attributes: Attributes::new(&emetadata)
                            };
                            file_links.insert(file_name.to_str().unwrap().to_string(), snapshot_sym_link);
                        } else {
                            let snapshot_file = SnapshotFile{
                                path: epath.to_path_buf(),
                                attributes: Attributes::new(&emetadata)
                            };
                            files.insert(file_name.to_str().unwrap().to_string(), snapshot_file);
                        }
                    }
                }
                None => (),
            }
        }
        Ok(SnapshotDir {
            path: path,
            attributes: Attributes::new(&metadata),
            subdirs: subdirs,
            files: files,
            file_links: file_links,
            subdir_links: subdir_links,
        })
    }

    pub fn new_empty(rootdir: &Path) -> io::Result<SnapshotDir> {
        let metadata = rootdir.metadata()?;
        let path = rootdir.canonicalize()?;

        let subdirs = HashMap::<String, SnapshotDir>::new();
        let files = HashMap::<String, SnapshotFile>::new();
        let file_links = HashMap::<String, SnapshotSymLink>::new();
        let subdir_links = HashMap::<String, SnapshotSymLink>::new();

        Ok(SnapshotDir {
            path: path,
            attributes: Attributes::new(&metadata),
            subdirs: subdirs,
            files: files,
            file_links: file_links,
            subdir_links: subdir_links,
        })
    }

    pub fn find_subdir(&self, abs_subdir_path: &PathBuf) -> Option<&SnapshotDir> {
        assert!(abs_subdir_path.is_absolute());
        match abs_subdir_path.strip_prefix(&self.path) {
            Ok(rel_path) => {
                if rel_path == PathBuf::from("") {
                    return Some(self)
                }
                let first_name = first_component_name(rel_path).to_string();
                match self.subdirs.get(&first_name) {
                    Some(sd) => sd.find_subdir(abs_subdir_path),
                    None => None,
                }
            },
            Err(_) => None
        }
    }

    pub fn find_or_add_subdir(&mut self, abs_subdir_path: &PathBuf) -> io::Result<&SnapshotDir> {
        assert!(abs_subdir_path.is_absolute());
        match abs_subdir_path.strip_prefix(&self.path) {
            Ok(rel_path) => {
                if rel_path == PathBuf::from("") {
                    return Ok(self)
                }
                let first_name = first_component_name(rel_path).to_string();
                if !self.subdirs.contains_key(&first_name) {
                    let mut path_buf = PathBuf::new();
                    path_buf.push(self.path.clone());
                    path_buf.push(first_name.clone());
                    let snapshot_dir = SnapshotDir::new_empty(&path_buf)?;
                    self.subdirs.insert(first_name.clone(), snapshot_dir);
                }
                return self.subdirs.get_mut(&first_name).unwrap().find_or_add_subdir(abs_subdir_path)
            },
            Err(err) => panic!(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_component_name_works() {
        assert_eq!("first", first_component_name(Path::new("first/second")));
        assert_ne!("second", first_component_name(Path::new("first/second")))
    }

    #[test]
    fn serialization_works() {
        let p = Path::new("/home/peter/TEST");
        let sd = SnapshotDir::new(p).unwrap_or_else(|err| {
            panic!("bummer: {:?}", err);
        });
        let sd_str = serde_json::to_string(&sd).unwrap_or_else(|err| {
            panic!("double bummer: {:?}", err);
        });
        let sde: SnapshotDir = serde_json::from_str(&sd_str).unwrap_or_else(|err| {
            panic!("triple bummer: {:?}", err);
        });
        assert_eq!(sd, sde);
    }

    #[test]
    fn find_subdir_works() {
        let p = Path::new("/home/peter/TEST");
        let sd = SnapshotDir::new(p).unwrap_or_else(|err| {
            panic!("bummer: {:?}", err);
        });
        let sdp = PathBuf::from("/home/peter");
        assert_eq!(sd.find_subdir(&sdp), None);
        let sdp1 = PathBuf::from("/home/peter/TEST/patch_diff/gui");
        assert_ne!(sd.find_subdir(&sdp1), None);
    }

    #[test]
    fn find_or_add_subdir_works() {
        let mut sd = SnapshotDir::new_empty(Path::new("/")).unwrap();
        let p = PathBuf::from("/home/peter/TEST");
        {
            let ssd = sd.find_or_add_subdir(&p);
            assert!(ssd.is_ok());
            let ssd = ssd.unwrap();
            assert!(ssd.path == p.as_path());
        }
        let ssd = sd.find_subdir(&p);
        assert!(ssd.unwrap().path == p.as_path());
    }
}
