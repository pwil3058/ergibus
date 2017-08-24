#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

use std::io;
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::os::linux::fs::MetadataExt;
//use std::cmp::PartialEq;

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

//impl PartialEq for SnapshotDir {
    //fn eq(&self, other: &SnapshotDir) -> bool {
        //println!("dir: {:?}", self.path);
        //if self.path != other.path {
            //return false;
        //}
        //if self.attributes != other.attributes {
            //return false;
        //}
        //if self.files != other.files {
            //return false;
        //}
        //if self.file_links != other.file_links {
            //return false;
        //}
        //if self.subdir_links != other.subdir_links {
            //return false;
        //}
        //if self.subdirs != other.subdirs {
            //return false;
        //}
        //true
    //}
//}

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
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        let p = Path::new("..");
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
}
