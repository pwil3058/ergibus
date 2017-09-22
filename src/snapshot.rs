// Copyright 2017 Peter Williams <pwil3058@gmail.com>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Standard Library access
use std::collections::HashMap;
use std::fs::{self, Metadata, File, DirEntry};
use std::io::prelude::*;
use std::io;
use std::ops::{AddAssign};
use std::os::linux::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time;

// cargo.io crates acess
use chrono::prelude::*;
use regex;
use serde_json;
use snap;
use walkdir::{WalkDir, WalkDirIterator};

// local modules access
use archive::{self, Exclusions, ArchiveData, get_archive_data};
use content::{ContentMgmtKey, ContentManager};
use eerror::{EError, EResult};
use pathux::{first_subpath_as_string};
use report::{ignore_report_or_crash, report_broken_link_or_crash};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Attributes {
    st_mode: u32,
    st_size: u64,
}

impl Attributes {
    pub fn new(metadata: &Metadata) -> Attributes {
        Attributes{
            st_mode: metadata.st_mode(),
            st_size: metadata.st_size(),
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
            if let Err(err) = content_mgr.release_contents(&file_data.content_token) {
                panic!("{:?}: line {:?}: token error: {:?}", file!(), line!(), err);
            };
        }
        for subdir in self.subdirs.values() {
            subdir.release_contents(content_mgr);
        }
    }

    #[test]
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
                match self.subdirs.get_mut(&first_name) {
                    Some(subdir) => subdir.find_or_add_subdir(abs_subdir_path),
                    None => panic!("{:?}: line {:?}", file!(), line!())
                }
            },
            Err(err) => panic!("{:?}: line {:?}: {:?}", file!(), line!(), err),
        }
    }

    fn populate(&mut self, exclusions: &Exclusions, content_mgr: &ContentManager) -> (FileStats, SymLinkStats) {
        let mut file_stats = FileStats{file_count: 0, byte_count: 0};
        let mut sym_link_stats = SymLinkStats{dir_sym_link_count: 0, file_sym_link_count: 0};
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
                                        file_stats += self.add_file(&entry, &content_mgr);
                                    } else if e_type.is_symlink() {
                                        if exclusions.is_excluded_file(&entry.path()) {
                                            continue
                                        }
                                        sym_link_stats += self.add_symlink(&entry);
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
        (file_stats, sym_link_stats)
    }

    fn add_file(&mut self, dir_entry: &fs::DirEntry, content_mgr: &ContentManager) -> FileStats {
        let file_name = dir_entry.file_name().into_string().unwrap_or_else(
            |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
        );
        if self.files.contains_key(&file_name) {
            return FileStats{file_count: 0, byte_count: 0}
        }
        let attributes = match dir_entry.metadata() {
            Ok(ref metadata) => Attributes::new(metadata),
            Err(err) => {
                ignore_report_or_crash(&err, &dir_entry.path());
                return FileStats{file_count: 0, byte_count: 0}
            }
        };
        let content_token = match content_mgr.store_file_contents(&dir_entry.path()) {
            Ok(ct) => ct,
            Err(err) => {
                match err {
                    EError::ContentStoreIOError(io_err) => {
                        ignore_report_or_crash(&io_err, &dir_entry.path());
                        return FileStats{file_count: 0, byte_count: 0}
                    },
                    _ => panic!("{:?}: line {:?}: should not happen: {:?}", file!(), line!(), err)
                }
            }
        };
        let file_stats = FileStats{file_count: 1, byte_count: attributes.st_size};
        self.files.insert(file_name, FileData{attributes, content_token});
        file_stats
    }

    fn add_symlink(&mut self, dir_entry: &fs::DirEntry) -> SymLinkStats {
        let file_name = dir_entry.file_name().into_string().unwrap_or_else(
            |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
        );
        if self.file_links.contains_key(&file_name) || self.subdir_links.contains_key(&file_name) {
            return SymLinkStats{dir_sym_link_count: 0, file_sym_link_count: 0}
        }
        let attributes = match dir_entry.metadata() {
            Ok(ref metadata) => Attributes::new(metadata),
            Err(err) => {
                ignore_report_or_crash(&err, &dir_entry.path());
                return SymLinkStats{dir_sym_link_count: 0, file_sym_link_count: 0}
            }
        };
        let link_target = match dir_entry.path().read_link() {
            Ok(lt) => lt,
            Err(err) => {
                ignore_report_or_crash(&err, &dir_entry.path());
                return SymLinkStats{dir_sym_link_count: 0, file_sym_link_count: 0}
            }
        };
        let abs_target_path = match self.path.join(link_target.clone()).canonicalize() {
            Ok(atp) => atp,
            Err(ref err) => {
                report_broken_link_or_crash(err, &dir_entry.path(), &link_target);
                return SymLinkStats{dir_sym_link_count: 0, file_sym_link_count: 0}
            }
        };
        if abs_target_path.is_file() {
            self.file_links.insert(file_name, LinkData{attributes, link_target});
            return SymLinkStats{dir_sym_link_count: 0, file_sym_link_count: 1};
        } else if abs_target_path.is_dir() {
            self.subdir_links.insert(file_name, LinkData{attributes, link_target});
            return SymLinkStats{dir_sym_link_count: 1, file_sym_link_count: 0};
        }
        SymLinkStats{dir_sym_link_count: 0, file_sym_link_count: 0}
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct FileStats {
    file_count: u64,
    byte_count: u64,
}

impl AddAssign for FileStats {
    fn add_assign(&mut self, other: FileStats) {
        *self = FileStats {
            file_count: self.file_count + other.file_count,
            byte_count: self.byte_count + other.byte_count,
        };
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct SymLinkStats {
    dir_sym_link_count: u64,
    file_sym_link_count: u64,
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
struct SnapshotPersistentData {
    root_dir: SnapshotDir,
    content_mgmt_key: ContentMgmtKey,
    archive_name: String,
    started_create: time::SystemTime,
    finished_create: time::SystemTime,
    file_stats: FileStats,
    sym_link_stats: SymLinkStats,
}

impl SnapshotPersistentData {
    fn new(archive_name: &str, rmk: &ContentMgmtKey) -> SnapshotPersistentData {
        let sd = SnapshotDir::new(None).unwrap_or_else(
            |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
        );
        SnapshotPersistentData{
            root_dir: sd,
            content_mgmt_key: rmk.clone(),
            archive_name: archive_name.to_string(),
            started_create: time::SystemTime::now(),
            finished_create: time::SystemTime::now(),
            file_stats: FileStats{file_count: 0, byte_count: 0},
            sym_link_stats: SymLinkStats{dir_sym_link_count: 0, file_sym_link_count: 0},
        }
    }

    fn serialize(&self) -> EResult<String> {
        match serde_json::to_string(self) {
            Ok(string) => Ok(string),
            Err(err) => Err(EError::SnapshotSerializeError(err)),
        }
    }

    fn release_contents(&self) {
        let content_mgr = self.content_mgmt_key.open_content_manager(true).unwrap_or_else(
            |err| panic!("{:?}: line {:?}: open content manager: {:?}", file!(), line!(), err)
        );
        self.root_dir.release_contents(&content_mgr);
    }

    fn add_dir(&mut self, abs_dir_path: &Path, exclusions: &Exclusions) -> io::Result<()> {
        let dir = self.root_dir.find_or_add_subdir(&abs_dir_path)?;
        let content_mgr = self.content_mgmt_key.open_content_manager(true).unwrap_or_else(
            |err| panic!("{:?}: line {:?}: open content manager: {:?}", file!(), line!(), err)
        );
        dir.populate(exclusions, &content_mgr);
        for entry in WalkDir::new(abs_dir_path).into_iter().filter_entry(|e| e.file_type().is_dir()) {
            match entry {
                Ok(e_data) => {
                    let e_path = e_data.path();
                    if exclusions.is_excluded_dir(e_path) {
                        continue
                    }
                    match dir.find_or_add_subdir(e_path) {
                        Ok(sub_dir) => {
                            let (file_stats, sym_link_stats) = sub_dir.populate(exclusions, &content_mgr);
                            self.file_stats += file_stats;
                            self.sym_link_stats += sym_link_stats
                        },
                        Err(err) => ignore_report_or_crash(&err, &e_path)
                    }
                },
                Err(err) => {
                    let path_buf = match err.path() {
                        Some(path) => path.to_path_buf(),
                        None => panic!("{:?}: line {:?}", file!(), line!())
                    };
                    let io_error = io::Error::from(err);
                    ignore_report_or_crash(&io_error, &path_buf);
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
        let dt = DateTime::<Local>::from(self.finished_create);
        PathBuf::from(format!("{}", dt.format("%Y-%m-%d-%H-%M-%S%z")))
    }

    fn write_to_dir(&self, dir_path: &Path) -> EResult<PathBuf> {
        let file_name = self.file_name();
        let path = dir_path.join(file_name);
        let file = File::create(&path).map_err(|err| EError::SnapshotWriteIOError(err, path.to_path_buf()))?;
        let json_text = self.serialize()?;
        let mut snappy_wtr = snap::Writer::new(file);
        snappy_wtr.write_all(json_text.as_bytes()).map_err(|err| EError::SnapshotWriteIOError(err, path.to_path_buf()))?;
        Ok(path)
    }
}

// Doing this near where the file names are constructed for programming convenience
lazy_static!{
    static ref SS_FILE_NAME_RE: regex::Regex = regex::Regex::new(r"^(\d{4})-(\d{2})-(\d{2})-(\d{2})-(\d{2})-(\d{2})[+-](\d{4})$").unwrap();
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
    let dir_entries = fs::read_dir(dir_path).map_err(|err| EError::SnapshotDirIOError(err, dir_path.to_path_buf()))?;
    let mut v = Vec::new();
    for entry_or_err in dir_entries {
        match entry_or_err {
            Ok(entry) => if entry_is_ss_file(&entry) {
                v.push(entry);
            },
            Err(_) => ()
        }
    }
    Ok(v)
}

impl SnapshotPersistentData {
    fn from_file(file_path: &Path) -> EResult<SnapshotPersistentData> {
        match File::open(file_path) {
            Ok(file) => {
                let mut spd_str = String::new();
                let mut snappy_rdr = snap::Reader::new(file);
                match snappy_rdr.read_to_string(&mut spd_str) {
                    Err(err) => return Err(EError::SnapshotReadIOError(err, file_path.to_path_buf())),
                    _ => ()
                };
                let spde = serde_json::from_str::<SnapshotPersistentData>(&spd_str);
                match spde {
                    Ok(snapshot_persistent_data) => Ok(snapshot_persistent_data),
                    Err(err) => Err(EError::SnapshotReadJsonError(err, file_path.to_path_buf()))
                }
            },
            Err(err) => Err(EError::SnapshotReadIOError(err, file_path.to_path_buf()))
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
            self.release_snapshot();
        }
    }
}

impl SnapshotGenerator {
    pub fn new(archive_name: &str) -> EResult<SnapshotGenerator> {
        let archive_data = get_archive_data(archive_name)?;
        let snapshot: Option<SnapshotPersistentData> = None;
        Ok(SnapshotGenerator{ snapshot, archive_data })
    }

    #[test]
    pub fn snapshot_available(&self) -> bool {
        self.snapshot.is_some()
    }

    fn generate_snapshot(&mut self) -> time::Duration {
        if self.snapshot.is_some() {
            // This snapshot is being thrown away so we release its contents
            self.release_snapshot();
        }
        let mut snapshot = SnapshotPersistentData::new(&self.archive_data.name, &self.archive_data.content_mgmt_key);
        for abs_path in self.archive_data.includes.iter() {
            if abs_path.is_dir() {
                if let Err(err) = snapshot.add_dir(&abs_path, &self.archive_data.exclusions) {
                    ignore_report_or_crash(&err, &abs_path);
                };
            }
        }
        snapshot.finished_create = time::SystemTime::now();
        let duration = snapshot.creation_duration();
        self.snapshot = Some(snapshot);
        duration
    }

    #[test]
    pub fn generation_duration(&self) -> EResult<time::Duration> {
        match self.snapshot {
            Some(ref snapshot) => Ok(snapshot.creation_duration()),
            None => Err(EError::NoSnapshotAvailable)
        }
    }

    fn release_snapshot(&mut self) {
        match self.snapshot {
            Some(ref snapshot) => snapshot.release_contents(),
            None => ()
        }
        self.snapshot = None;
    }

    fn write_snapshot(&mut self) -> EResult<PathBuf> {
        let file_path = match self.snapshot {
            Some(ref snapshot) => {
                snapshot.write_to_dir(&self.archive_data.snapshot_dir_path)?
            },
            None => return Err(EError::NoSnapshotAvailable)
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
                        Ok(_) => Err(EError::SnapshotMismatch(file_path.to_path_buf())),
                        Err(err) => Err(EError::SnapshotMismatchDirty(err, file_path.to_path_buf()))
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

pub fn generate_snapshot(archive_name: &str) -> EResult<()> {
    let mut sg = SnapshotGenerator::new(archive_name)?;
    sg.generate_snapshot();
    sg.write_snapshot()?;
    Ok(())
}

pub fn delete_snapshot_file(ss_file_path: &Path) -> EResult<()> {
    let snapshot = SnapshotPersistentData::from_file(ss_file_path)?;
    fs::remove_file(ss_file_path).map_err(|err| EError::SnapshotDeleteIOError(err, ss_file_path.to_path_buf()))?;
    snapshot.release_contents();
    Ok(())
}

pub fn get_snapshot_names_in_dir(dir_path: &Path) -> EResult<Vec<String>> {
    let entries = get_ss_entries_in_dir(dir_path)?;
    let mut v = Vec::new();
    for entry in entries {
        match entry.file_name().to_str() {
            Some(file_name) => v.push(file_name.to_string()),
            None => panic!("{:?}: line {:?}: {:?}", file!(), line!(), entry.file_name())
        }
    };
    Ok(v)
}

pub fn get_snapshot_names_for_archive(archive_name: &str) -> EResult<Vec<String>> {
    let snapshot_dir_path = archive::get_archive_snapshot_dir_path(archive_name)?;
    get_snapshot_names_in_dir(&snapshot_dir_path)
}

#[cfg(test)]
mod tests {
    use std::env;
    use fs2::FileExt;
    use tempdir::TempDir;
    use super::*;
    use content;
    use archive;

    #[test]
    fn test_ssf_regex() {
        assert!(SS_FILE_NAME_RE.is_match("1027-09-14-20-20-59-1000"));
        assert!(SS_FILE_NAME_RE.is_match("1027-09-14-20-20-59+1000"));
    }

    #[test]
    fn find_or_add_subdir_works() {
        let mut sd = SnapshotDir::new(None).unwrap_or_else(
            |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
        );
        let p = PathBuf::from("/home/peter/TEST");
        {
            let ssd = sd.find_or_add_subdir(&p);
            assert!(ssd.is_ok());
            let ssd = ssd.unwrap_or_else(
                |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
            );
            assert!(ssd.path == p.as_path());
        }
        let ssd = match sd.find_subdir(&p) {
            Some(ssd) => ssd,
            None => panic!("{:?}: line {:?}", file!(), line!())
        };
        assert!(ssd.path == p.as_path());
        let sdp = PathBuf::from("/home/peter");
        let ssd = match sd.find_subdir(&sdp) {
            Some(ssd) => ssd,
            None => panic!("{:?}: line {:?}", file!(), line!())
        };
        assert_eq!(ssd.path, sdp.as_path());
        let sdp1 = PathBuf::from("/home/peter/TEST/patch_diff/gui");
        assert_eq!(sd.find_subdir(&sdp1), None);
    }

    #[test]
    fn test_write_snapshot() {
        let file = fs::OpenOptions::new().write(true).open("./test_lock_file").unwrap_or_else(
            |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
        );
        if let Err(err) = file.lock_exclusive() {
            panic!("lock failed: {:?}", err);
        };
        let dir = TempDir::new("SS_TEST").unwrap_or_else(
            |err| panic!("open temp dir failed: {:?}", err)
        );
        env::set_var("ERGIBUS_CONFIG_DIR", dir.path().join("config"));
        let data_dir = dir.path().join("data");
        let data_dir_str = match data_dir.to_str() {
            Some(data_dir_str) => data_dir_str,
            None => panic!("{:?}: line {:?}", file!(), line!())
        };
        if let Err(err) = content::create_new_repo("test_repo", data_dir_str, "Sha1") {
            panic!("new repo: {:?}", err);
        }
        let inclusions = vec!["~/Documents".to_string(), "~/Downloads".to_string()];
        let dir_exclusions = vec!["lost+found".to_string()];
        let file_exclusions = vec!["*.iso".to_string()];
        if let Err(err) = archive::create_new_archive("test_ss", "test_repo", data_dir_str, inclusions, dir_exclusions, file_exclusions) {
            panic!("new archive: {:?}", err);
        }
        { // need this to let sg finish before the temporary directory is destroyed
            let mut sg = match SnapshotGenerator::new("test_ss") {
                Ok(snapshot_generator) => snapshot_generator,
                Err(err) => panic!("new SG: {:?}", err)
            };
            sg.generate_snapshot();
            println!("Generating for {:?} took {:?}", "test_ss", sg.generation_duration());
            assert!(sg.snapshot_available());
            let result = sg.write_snapshot();
            assert!(result.is_ok());
            assert!(!sg.snapshot_available());
            match result {
                Ok(ref ss_file_path) => {
                    match fs::metadata(ss_file_path) {
                        Ok(metadata) => println!("{:?}: {:?}", ss_file_path, metadata.st_size()),
                        Err(err) => panic!("Error getting size data: {:?}: {:?}", ss_file_path, err)
                    };
                    match SnapshotPersistentData::from_file(ss_file_path) {
                        Ok(ss) => println!("{:?}: {:?} {:?}", ss.archive_name, ss.file_stats, ss.sym_link_stats),
                        Err(err) => panic!("Error reading: {:?}: {:?}", ss_file_path, err)
                    };

                },
                Err(err) => panic!("{:?}", err)
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
