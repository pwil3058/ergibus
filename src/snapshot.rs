// Standard Library access
use std::collections::HashMap;
use std::collections::hash_map::Iter;
use std::error::Error;
use std::fs::{self, Metadata, File};
use std::io::prelude::*;
use std::io::{self, ErrorKind};
use std::os::linux::fs::MetadataExt;
use std::path::{Path, PathBuf, Component};
use std::time;

// cargo.io crates acess
use chrono::prelude::*;
use serde;
use serde_json;
use walkdir::{WalkDir, WalkDirIterator};

// local crates access
use content::{ContentMgmtKey, ContentManager, HashAlgorithm, ContentError};
use pathux::{split_abs_path, split_rel_path, first_subpath_as_string};
use report::{ignore_report_or_crash, report_broken_link_or_crash};

#[derive(Debug)]
enum SSError {
    NoSnapshotAvailable,
    SnapshotMismatch,
    SnapshotMismatchDirty(io::Error),
    IOError(io::Error),
    JsonError(serde_json::Error),
    SnapshotReadIOError(io::Error),
    SnapshotReadJsonError(serde_json::Error),
}

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
struct FileData {
    attributes: Attributes,
    content_token: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct LinkData {
    attributes: Attributes,
    link_target: PathBuf,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct SnapshotDir {
    path: PathBuf,
    attributes: Attributes,
    subdirs: HashMap<String, SnapshotDir>,
    files: HashMap<String, FileData>,
    file_links: HashMap<String, LinkData>,
    subdir_links: HashMap<String, LinkData>,
}

impl SnapshotDir {
    fn new(opt_rootdir: Option<&Path>) -> io::Result<SnapshotDir> {
        let rootdir = match opt_rootdir {
            Some(p) => p,
            None => Path::new("/"),
        };
        let metadata = rootdir.metadata()?;
        let path = rootdir.canonicalize()?;

        let subdirs = HashMap::<String, SnapshotDir>::new();
        let files = HashMap::<String, FileData>::new();
        let file_links = HashMap::<String, LinkData>::new();
        let subdir_links = HashMap::<String, LinkData>::new();

        Ok(SnapshotDir {
            path: path,
            attributes: Attributes::new(&metadata),
            subdirs: subdirs,
            files: files,
            file_links: file_links,
            subdir_links: subdir_links,
        })
    }

    fn release_contents(&self, content_mgr: &ContentManager) {
        for file_data in self.files.values() {
            content_mgr.release_contents(&file_data.content_token).unwrap();
        }
        for subdir in self.subdirs.values() {
            subdir.release_contents(content_mgr);
        }
    }

    fn find_subdir(&self, abs_subdir_path: &PathBuf) -> Option<&SnapshotDir> {
        assert!(abs_subdir_path.is_absolute());
        match abs_subdir_path.strip_prefix(&self.path) {
            Ok(rel_path) => {
                let first_name = match first_subpath_as_string(rel_path) {
                    Some(fname) => fname,
                    None => return Some(self)
                };
                match self.subdirs.get(&first_name) {
                    Some(sd) => sd.find_subdir(abs_subdir_path),
                    None => None,
                }
            },
            Err(_) => None
        }
    }

    fn find_or_add_subdir(&mut self, abs_subdir_path: &Path) -> io::Result<&mut SnapshotDir> {
        assert!(abs_subdir_path.is_absolute());
        match abs_subdir_path.strip_prefix(&self.path.clone()) {
            Ok(rel_path) => {
                let first_name = match first_subpath_as_string(rel_path) {
                    Some(fname) => fname,
                    None => return Ok(self)
                };
                if !self.subdirs.contains_key(&first_name) {
                    let mut path_buf = PathBuf::new();
                    path_buf.push(self.path.clone());
                    path_buf.push(first_name.clone());
                    let snapshot_dir = SnapshotDir::new(Some(&path_buf))?;
                    self.subdirs.insert(first_name.clone(), snapshot_dir);
                }
                return self.subdirs.get_mut(&first_name).unwrap().find_or_add_subdir(abs_subdir_path)
            },
            Err(err) => panic!(err),
        }
    }

    fn populate(&mut self, exclusions: &Exclusions, content_mgr: &ContentManager) {
        match fs::read_dir(&self.path) {
            Ok(entries) => {
                for entry_or_err in entries {
                    match entry_or_err {
                        Ok(entry) => {
                            match entry.file_type() {
                                Ok(e_type) => {
                                    if e_type.is_file() {
                                        if exclusions.is_excluded_file(&entry.path()) {
                                            continue
                                        }
                                        self.add_file(&entry, &content_mgr);
                                    } else if e_type.is_symlink() {
                                        if exclusions.is_excluded_file(&entry.path()) {
                                            continue
                                        }
                                        self.add_symlink(&entry);
                                    }
                                },
                                Err(err) => ignore_report_or_crash(&err, &self.path)
                            }
                        },
                        Err(err) => ignore_report_or_crash(&err, &self.path)
                    }
                }
            },
            Err(err) => ignore_report_or_crash(&err, &self.path)
        };
    }

    fn add_file(&mut self, dir_entry: &fs::DirEntry, content_mgr: &ContentManager) {
        let file_name = dir_entry.file_name().into_string().unwrap();
        if self.files.contains_key(&file_name) {
            return
        }
        let attributes = match dir_entry.metadata() {
            Ok(ref metadata) => Attributes::new(metadata),
            Err(err) => {
                ignore_report_or_crash(&err, &dir_entry.path());
                return
            }
        };
        let content_token = match content_mgr.store_file_contents(&dir_entry.path()) {
            Ok(ct) => ct,
            Err(err) => {
                match err {
                    ContentError::FileSystemError(io_err) => {
                        ignore_report_or_crash(&io_err, &dir_entry.path());
                        return
                    },
                    _ => panic!("should not happen")
                }
            }
        };
        self.files.insert(file_name, FileData{attributes, content_token});
    }

    fn add_symlink(&mut self, dir_entry: &fs::DirEntry) {
        let file_name = dir_entry.file_name().into_string().unwrap();
        if self.file_links.contains_key(&file_name) || self.subdir_links.contains_key(&file_name) {
            return
        }
        let attributes = match dir_entry.metadata() {
            Ok(ref metadata) => Attributes::new(metadata),
            Err(err) => {
                ignore_report_or_crash(&err, &dir_entry.path());
                return
            }
        };
        let link_target = match dir_entry.path().read_link() {
            Ok(lt) => lt,
            Err(err) => {
                ignore_report_or_crash(&err, &dir_entry.path());
                return
            }
        };
        let abs_target_path = match self.path.join(link_target.clone()).canonicalize() {
            Ok(atp) => atp,
            Err(ref err) => {
                report_broken_link_or_crash(err, &dir_entry.path(), &link_target);
                return
            }
        };
        if abs_target_path.is_file() {
            self.file_links.insert(file_name, LinkData{attributes, link_target});
        } else if abs_target_path.is_dir() {
            self.subdir_links.insert(file_name, LinkData{attributes, link_target});
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct SnapshotPersistentData {
    root_dir: SnapshotDir,
    content_mgmt_key: ContentMgmtKey,
    started_create: time::SystemTime,
    finished_create: time::SystemTime,
}

impl SnapshotPersistentData {
    fn new(rmk: &ContentMgmtKey) -> SnapshotPersistentData {
        let sd = SnapshotDir::new(None).unwrap();
        SnapshotPersistentData{
            root_dir: sd,
            content_mgmt_key: rmk.clone(),
            started_create: time::SystemTime::now(),
            finished_create: time::SystemTime::now(),
        }
    }

    fn serialize(&self) -> Result<String, SSError> {
        match serde_json::to_string(self) {
            Ok(string) => Ok(string),
            Err(err) => Err(SSError::JsonError(err)),
        }
    }

    fn release_contents(&self) {
        let content_mgr = ContentManager::new(&self.content_mgmt_key, true);
        self.root_dir.release_contents(&content_mgr);
    }

    fn add_dir(&mut self, abs_dir_path: &Path, exclusions: &Exclusions) -> io::Result<()> {
        let dir = self.root_dir.find_or_add_subdir(&abs_dir_path)?;
        let content_mgr = ContentManager::new(&self.content_mgmt_key, true);
        dir.populate(exclusions, &content_mgr);
        for entry in WalkDir::new(abs_dir_path).into_iter().filter_entry(|e| e.file_type().is_dir()) {
            match entry {
                Ok(e_data) => {
                    let e_path = e_data.path();
                    if exclusions.is_excluded_dir(e_path) {
                        continue
                    }
                    match dir.find_or_add_subdir(e_path) {
                        Ok(sub_dir) => sub_dir.populate(exclusions, &content_mgr),
                        Err(err) => ignore_report_or_crash(&err, &e_path)
                    }
                },
                Err(err) => {
                    let path = err.path().unwrap().to_path_buf();
                    let io_error = io::Error::from(err);
                    ignore_report_or_crash(&io_error, &path);
                },
            }
        }
        Ok(())
    }

    fn creation_duration(&self) -> time::Duration {
        match self.finished_create.duration_since(self.started_create) {
            Ok(duration) => duration,
            Err(_) => time::Duration::new(0, 0)
        }
    }

    fn file_name(&self) -> PathBuf {
        let dt = DateTime::<Utc>::from(self.finished_create);
        PathBuf::from(format!("{}", dt.format("%Y-%m-%d-%H-%M-%SZ")))
    }

    fn write_to_dir(&self, dir_path: &Path) -> Result<PathBuf, SSError> {
        let file_name = self.file_name();
        let path = dir_path.join(file_name);
        let mut file = File::create(&path).map_err(|err| SSError::IOError(err))?;
        let json_text = self.serialize()?;
        file.write_all(json_text.as_bytes()).map_err(|err| SSError::IOError(err))?;
        Ok(path)
    }
}

impl SnapshotPersistentData {
    fn from_file(file_path: &Path) -> Result<SnapshotPersistentData, SSError> {
        match File::open(file_path) {
            Ok(mut file) => {
                let mut spd_str = String::new();
                match file.read_to_string(&mut spd_str) {
                    Err(err) => return Err(SSError::SnapshotReadIOError(err)),
                    _ => ()
                };
                let spde = serde_json::from_str::<SnapshotPersistentData>(&spd_str);
                match spde {
                    Ok(snapshot_persistent_data) => Ok(snapshot_persistent_data),
                    Err(err) => Err(SSError::SnapshotReadJsonError(err))
                }
            },
            Err(err) => Err(SSError::SnapshotReadIOError(err))
        }
    }
}

#[derive(Debug)]
struct Exclusions {}

impl Exclusions {
    fn is_excluded_dir(&self, abs_dir_path: &Path) -> bool {
        return false;
    }

    fn is_excluded_file(&self, abs_file_path: &Path) -> bool {
        return false;
    }
}

#[derive(Debug)]
struct SnapshotGenerator {
    snapshot: Option<SnapshotPersistentData>,
    base_dir_path: PathBuf,
    exclusions: Exclusions,
    content_mgmt_key: ContentMgmtKey,
}

impl Drop for SnapshotGenerator {
    fn drop(&mut self) {
        if self.snapshot.is_some() {
            self.release_snapshot();
        }
    }
}

impl SnapshotGenerator {
    pub fn new(bdp: &Path, rmk: ContentMgmtKey) -> SnapshotGenerator {
        SnapshotGenerator {
            snapshot: None,
            base_dir_path: bdp.to_path_buf(),
            exclusions: Exclusions{},
            content_mgmt_key: rmk,
        }
    }

    fn snapshot_available(&self) -> bool {
        self.snapshot.is_some()
    }

    #[cfg(test)]
    fn serialised_snapshot(&self) -> Result<String, SSError> {
        match self.snapshot {
            Some(ref snapshot) => snapshot.serialize(),
            None => Err(SSError::NoSnapshotAvailable)
        }
    }

    #[cfg(test)]
    fn matches_snapshot(&self, snapshot: &SnapshotPersistentData) -> bool {
        match self.snapshot {
            Some(ref my_snapshot) => *my_snapshot == *snapshot,
            None => false
        }
    }

    fn generate_snapshot(&mut self) -> time::Duration {
        if self.snapshot.is_some() {
            // This snapshot is being thrown away so we release its contents
            self.release_snapshot();
        }
        let mut snapshot = SnapshotPersistentData::new(&self.content_mgmt_key);
        snapshot.add_dir(&self.base_dir_path, &self.exclusions);
        snapshot.finished_create = time::SystemTime::now();
        let duration = snapshot.creation_duration();
        self.snapshot = Some(snapshot);
        duration
    }

    fn generation_duration(&self) -> Result<time::Duration, SSError> {
        match self.snapshot {
            Some(ref snapshot) => Ok(snapshot.creation_duration()),
            None => Err(SSError::NoSnapshotAvailable)
        }
    }

    fn release_snapshot(&mut self) {
        match self.snapshot {
            Some(ref snapshot) => snapshot.release_contents(),
            None => ()
        }
        self.snapshot = None;
    }

    fn write_snapshot_to(&mut self, dir_path: &Path) -> Result<PathBuf, SSError> {
        let file_path = match self.snapshot {
            Some(ref snapshot) => {
                snapshot.write_to_dir(dir_path)?
            },
            None => return Err(SSError::NoSnapshotAvailable)
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
                        Ok(_) => Err(SSError::SnapshotMismatch),
                        Err(err) => Err(SSError::SnapshotMismatchDirty(err))
                    }
                }
            },
            Err(err) => {
                // The file is mangled so remove it
                match fs::remove_file(&file_path) {
                    Ok(_) => Err(err),
                    Err(_) => Err(err)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialization_works() {
        let content_mgmt_key = ContentMgmtKey::new_dummy();
        let p = Path::new(".").canonicalize().unwrap();
        let mut sg = SnapshotGenerator::new(&p, content_mgmt_key);
        sg.generate_snapshot();
        let spd_str = sg.serialised_snapshot().unwrap_or_else(|err| {
            panic!("double bummer: {:?}", err);
        });
        let spde: SnapshotPersistentData = serde_json::from_str(&spd_str).unwrap_or_else(|err| {
            panic!("triple bummer: {:?}", err);
        });
        assert!(sg.matches_snapshot(&spde));
    }

    #[test]
    fn find_or_add_subdir_works() {
        let mut sd = SnapshotDir::new(None).unwrap();
        let p = PathBuf::from("/home/peter/TEST");
        {
            let ssd = sd.find_or_add_subdir(&p);
            assert!(ssd.is_ok());
            let ssd = ssd.unwrap();
            assert!(ssd.path == p.as_path());
        }
        let ssd = sd.find_subdir(&p);
        assert!(ssd.unwrap().path == p.as_path());
        let sdp = PathBuf::from("/home/peter");
        assert_eq!(sd.find_subdir(&sdp).unwrap().path, sdp.as_path());
        let sdp1 = PathBuf::from("/home/peter/TEST/patch_diff/gui");
        assert_eq!(sd.find_subdir(&sdp1), None);
    }

    #[test]
    fn test_snapshot_creator() {
        let content_mgmt_key = ContentMgmtKey::new_dummy();
        let p = Path::new(".").canonicalize().unwrap();
        let mut sg = SnapshotGenerator::new(&p, content_mgmt_key);
        sg.generate_snapshot();
        assert!(sg.snapshot_available())
    }

    #[test]
    fn test_write_snapshot() {
        let content_mgmt_key = ContentMgmtKey::new_dummy();
        let p = Path::new(".").canonicalize().unwrap();
        let mut sg = SnapshotGenerator::new(&p, content_mgmt_key);
        sg.generate_snapshot();
        assert!(sg.snapshot_available());
        let result = sg.write_snapshot_to(Path::new("/home/peter/TEST/"));
        assert!(result.is_ok());
        assert!(!sg.snapshot_available());
    }
}
