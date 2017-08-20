use std::io;
use std::ffi::OsString;
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

struct SnapshotFile {
    path: PathBuf,
    attributes: Metadata,
}

struct SnapshotSymLink {
    path: PathBuf,
    attributes: Metadata,
}

struct SnapshotDir {
    path: PathBuf,
    attributes: Metadata,
    subdirs: HashMap<OsString, SnapshotDir>,
    files: HashMap<OsString, SnapshotFile>,
    file_links: HashMap<OsString, SnapshotSymLink>,
    subdir_links: HashMap<OsString, SnapshotSymLink>,
}

impl SnapshotDir {
    pub fn new(rootdir: &Path) -> io::Result<SnapshotDir> {
        let attributes = rootdir.metadata()?;
        let path = rootdir.canonicalize()?;

        let mut subdirs = HashMap::<OsString, SnapshotDir>::new();
        let mut files = HashMap::<OsString, SnapshotFile>::new();
        let mut file_links = HashMap::<OsString, SnapshotSymLink>::new();
        let mut subdir_links = HashMap::<OsString, SnapshotSymLink>::new();

        for entry in fs::read_dir(path.as_path())? {
            let entry = entry?;
            let epathbuf = entry.path();
            let epath = epathbuf.as_path();
            match epath.file_name() {
                Some(file_name) => {
                    let eattributes = epath.metadata()?;
                    if eattributes.is_dir() {
                        if eattributes.file_type().is_symlink() {
                            let snapshot_sym_link = SnapshotSymLink{
                                path: epath.to_path_buf(),
                                attributes: eattributes
                            };
                            subdir_links.insert(file_name.to_os_string(), snapshot_sym_link);
                        } else {
                            let snapshot_dir = match SnapshotDir::new(epath) {
                                Ok(ssd) => ssd,
                                Err(_) => continue,
                            };
                            subdirs.insert(file_name.to_os_string(), snapshot_dir);
                        }
                    } else {
                        if eattributes.file_type().is_symlink() {
                            let snapshot_sym_link = SnapshotSymLink{
                                path: epath.to_path_buf(),
                                attributes: eattributes
                            };
                            file_links.insert(file_name.to_os_string(), snapshot_sym_link);
                        } else {
                            let snapshot_file = SnapshotFile{
                                path: epath.to_path_buf(),
                                attributes: eattributes
                            };
                            files.insert(file_name.to_os_string(), snapshot_file);
                        }
                    }
                }
                None => (),
            }
        }
        Ok(SnapshotDir {
            path: path,
            attributes: attributes,
            subdirs: subdirs,
            files: files,
            file_links: file_links,
            subdir_links: subdir_links,
        })
    }
}

fn main() {
    let p = Path::new(".");
    let sd = match SnapshotDir::new(p) {
        Ok(ssd) => ssd,
        Err(_) => panic!("bummer")
    };
    let pc = p.canonicalize();
    println!("path = {:?} ({:?}) {:?} {:?}", p.to_str(), pc, sd.path, sd.attributes);
    println!("Hello, world!");
}
