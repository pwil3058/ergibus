use std::fs::{self, File};
use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};
use hostname;
use serde_yaml;
use users;
use walkdir;

use crate::snapshot::{ExtractionStats, SnapshotPersistentData};
use crate::{
    config,
    content::{content_repo_exists, get_content_mgmt_key, ContentMgmtKey},
    snapshot, EResult, Error,
};
use pw_pathux::expand_home_dir;
use std::convert::TryFrom;
use std::io::{stderr, ErrorKind};
use std::time;

#[derive(Debug)]
pub struct Exclusions {
    dir_globset: GlobSet,
    file_globset: GlobSet,
}

impl Exclusions {
    fn new(dir_patterns: &Vec<String>, file_patterns: &Vec<String>) -> EResult<Exclusions> {
        let mut dgs_builder = GlobSetBuilder::new();
        for pattern in dir_patterns {
            let glob = Glob::new(pattern).map_err(|err| Error::GlobError(err))?;
            dgs_builder.add(glob);
        }
        let dir_globset = dgs_builder.build().map_err(|err| Error::GlobError(err))?;

        let mut fgs_builder = GlobSetBuilder::new();
        for pattern in file_patterns {
            let glob = Glob::new(pattern).map_err(|err| Error::GlobError(err))?;
            fgs_builder.add(glob);
        }
        let file_globset = fgs_builder.build().map_err(|err| Error::GlobError(err))?;

        Ok(Exclusions {
            dir_globset,
            file_globset,
        })
    }

    pub fn is_non_excluded_dir(&self, dir_entry: &walkdir::DirEntry) -> bool {
        if dir_entry.file_type().is_dir() {
            if self.dir_globset.is_empty() {
                true
            } else if self.dir_globset.is_match(&dir_entry.file_name()) {
                false
            } else if self.dir_globset.is_match(&dir_entry.path()) {
                false
            } else {
                true
            }
        } else {
            false
        }
    }

    pub fn is_excluded_dir(&self, abs_dir_path: &Path) -> bool {
        if self.dir_globset.is_empty() {
            return false;
        } else if self.dir_globset.is_match(abs_dir_path) {
            return true;
        } else {
            let dir_name = match abs_dir_path.file_name() {
                Some(dir_name) => dir_name,
                None => panic!("{:?}: line {:?}", file!(), line!()),
            };
            return self.dir_globset.is_match(&dir_name);
        }
    }

    pub fn is_excluded_file(&self, abs_file_path: &Path) -> bool {
        if self.file_globset.is_empty() {
            return false;
        } else if self.file_globset.is_match(abs_file_path) {
            return true;
        } else {
            let file_name = match abs_file_path.file_name() {
                Some(file_name) => file_name,
                None => panic!("{:?}: line {:?}", file!(), line!()),
            };
            return self.file_globset.is_match(&file_name);
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct ArchiveSpec {
    content_repo_name: String,
    snapshot_dir_path: String,
    inclusions: Vec<String>,
    dir_exclusions: Vec<String>,
    file_exclusions: Vec<String>,
}

fn get_archive_spec_file_path(archive_name: &str) -> PathBuf {
    config::get_archive_config_dir_path().join(archive_name)
}

fn read_archive_spec(archive_name: &str) -> EResult<ArchiveSpec> {
    let spec_file_path = get_archive_spec_file_path(archive_name);
    let spec_file = File::open(&spec_file_path).map_err(|err| match err.kind() {
        ErrorKind::NotFound => Error::ArchiveUnknown(archive_name.to_string()),
        _ => Error::ArchiveReadError(err, spec_file_path.clone()),
    })?;
    let spec: ArchiveSpec = serde_yaml::from_reader(&spec_file)
        .map_err(|err| Error::ArchiveYamlReadError(err, archive_name.to_string()))?;
    Ok(spec)
}

fn write_archive_spec(
    archive_name: &str,
    archive_spec: &ArchiveSpec,
    overwrite: bool,
) -> EResult<()> {
    let spec_file_path = get_archive_spec_file_path(archive_name);
    if !overwrite && spec_file_path.exists() {
        return Err(Error::ArchiveExists(archive_name.to_string()));
    }
    match spec_file_path.parent() {
        Some(config_dir_path) => {
            if !config_dir_path.exists() {
                fs::create_dir_all(&config_dir_path)
                    .map_err(|err| Error::ArchiveWriteError(err, config_dir_path.to_path_buf()))?;
            }
        }
        None => (),
    }
    let spec_file = File::create(&spec_file_path)
        .map_err(|err| Error::ArchiveWriteError(err, spec_file_path.clone()))?;
    serde_yaml::to_writer(&spec_file, archive_spec)
        .map_err(|err| Error::ArchiveYamlWriteError(err, archive_name.to_string()))?;
    Ok(())
}

#[derive(Debug)]
pub struct ArchiveData {
    pub name: String,
    pub content_mgmt_key: ContentMgmtKey,
    pub snapshot_dir_path: PathBuf,
    pub includes: Vec<PathBuf>,
    pub exclusions: Exclusions,
}

pub fn create_new_archive(
    name: &str,
    content_repo_name: &str,
    location: &str,
    inclusions: &[String],
    dir_exclusions: &[String],
    file_exclusions: &[String],
) -> EResult<()> {
    if get_archive_spec_file_path(name).exists() {
        return Err(Error::ArchiveExists(name.to_string()));
    }
    if !content_repo_exists(content_repo_name) {
        return Err(Error::UnknownRepo(content_repo_name.to_string()));
    }
    for pattern in dir_exclusions.iter() {
        let _glob = Glob::new(&pattern).map_err(|err| Error::GlobError(err))?;
    }
    for pattern in file_exclusions.iter() {
        let _glob = Glob::new(&pattern).map_err(|err| Error::GlobError(err))?;
    }
    let mut snapshot_dir_path = PathBuf::from(location);
    snapshot_dir_path.push("ergibus");
    snapshot_dir_path.push("archives");
    match hostname::get_hostname() {
        Some(hostname) => snapshot_dir_path.push(hostname),
        None => (),
    };
    match users::get_current_username() {
        Some(user_name) => snapshot_dir_path.push(user_name),
        None => (),
    };
    snapshot_dir_path.push(name);
    fs::create_dir_all(&snapshot_dir_path)
        .map_err(|err| Error::ArchiveWriteError(err, snapshot_dir_path.clone()))?;
    let sdp_str = match snapshot_dir_path.to_str() {
        Some(sdp_ostr) => sdp_ostr.to_string(),
        None => panic!("{:?}: line {:?}", file!(), line!()),
    };
    let spec = ArchiveSpec {
        content_repo_name: content_repo_name.to_string(),
        snapshot_dir_path: sdp_str,
        inclusions: inclusions.to_vec(),
        dir_exclusions: dir_exclusions.to_vec(),
        file_exclusions: file_exclusions.to_vec(),
    };
    write_archive_spec(name, &spec, false)?;
    Ok(())
}

pub fn delete_archive(archive_name: &str) -> EResult<()> {
    let snapshot_dir = ArchiveSnapshotDir::try_from(archive_name)?;
    let spec_file_path = get_archive_spec_file_path(archive_name);
    fs::remove_file(&spec_file_path)?;
    snapshot_dir.delete()
}

pub fn get_archive_data(archive_name: &str) -> EResult<ArchiveData> {
    let archive_spec = read_archive_spec(archive_name)?;
    let name = archive_name.to_string();
    let content_mgmt_key = get_content_mgmt_key(&archive_spec.content_repo_name)?;
    let snapshot_dir_path = PathBuf::from(&archive_spec.snapshot_dir_path)
        .canonicalize()
        .map_err(|err| {
            Error::ArchiveDirError(err, PathBuf::from(&archive_spec.snapshot_dir_path))
        })?;
    let mut includes = Vec::new();
    for inclusion in archive_spec.inclusions {
        let included_file_path = if inclusion.starts_with("~") {
            match expand_home_dir(&PathBuf::from(inclusion)) {
                Some(expanded_path) => expanded_path,
                None => panic!(
                    "{:?}: line {:?}: home dir expansion failed",
                    file!(),
                    line!()
                ),
            }
        } else {
            let path_buf = PathBuf::from(inclusion);
            if path_buf.is_relative() {
                return Err(Error::RelativeIncludePath(
                    path_buf,
                    archive_name.to_string(),
                ));
            };
            path_buf
        };
        includes.push(included_file_path);
    }
    let exclusions = Exclusions::new(&archive_spec.dir_exclusions, &archive_spec.file_exclusions)?;

    Ok(ArchiveData {
        name,
        content_mgmt_key,
        snapshot_dir_path,
        includes,
        exclusions,
    })
}

// for read only snapshot actions we only need the snapshot directory path
// as the content manager key data is in the snapshot file.
// NB: this means that we can use snapshots even if the configuration
// data has been lost due to a file system failure (but in that case
// the user will have to browse the file system to find the snapshots).
pub fn get_archive_snapshot_dir_path(archive_name: &str) -> EResult<PathBuf> {
    let archive_spec = read_archive_spec(archive_name)?;
    PathBuf::from(&archive_spec.snapshot_dir_path)
        .canonicalize()
        .map_err(|err| Error::ArchiveDirError(err, PathBuf::from(&archive_spec.snapshot_dir_path)))
}

pub fn get_archive_names() -> Vec<String> {
    let mut names = Vec::new();
    if let Ok(dir_entries) = fs::read_dir(config::get_archive_config_dir_path()) {
        for entry_or_err in dir_entries {
            if let Ok(entry) = entry_or_err {
                let path = entry.path();
                if path.is_file() {
                    if let Some(file_name) = path.file_name() {
                        if let Some(file_name) = file_name.to_str() {
                            names.push(file_name.to_string());
                        }
                    }
                }
            }
        }
    };
    names
}

#[derive(Debug, Clone)]
pub enum ArchiveNameOrDirPath {
    ArchiveName(String),
    DirPath(PathBuf),
}

impl From<&str> for ArchiveNameOrDirPath {
    fn from(name: &str) -> Self {
        ArchiveNameOrDirPath::ArchiveName(name.to_string())
    }
}

impl From<&Path> for ArchiveNameOrDirPath {
    fn from(path: &Path) -> Self {
        ArchiveNameOrDirPath::DirPath(path.to_path_buf())
    }
}

#[derive(Debug)]
pub struct ArchiveSnapshotDir {
    id: ArchiveNameOrDirPath,
    dir_path: PathBuf,
}

impl TryFrom<&str> for ArchiveSnapshotDir {
    type Error = crate::Error;

    fn try_from(name: &str) -> Result<Self, Self::Error> {
        let id = ArchiveNameOrDirPath::from(name);
        let dir_path = get_archive_snapshot_dir_path(name)?;
        Ok(Self { id, dir_path })
    }
}

impl TryFrom<&Path> for ArchiveSnapshotDir {
    type Error = crate::Error;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let id = ArchiveNameOrDirPath::from(path);
        let dir_path = PathBuf::from(path)
            .canonicalize()
            .map_err(|err| Error::ArchiveDirError(err, PathBuf::from(path)))?;
        Ok(Self { id, dir_path })
    }
}

impl ArchiveSnapshotDir {
    pub fn id(&self) -> &ArchiveNameOrDirPath {
        &self.id
    }

    pub fn delete(&self) -> EResult<()> {
        let snapshot_paths = self.get_snapshot_paths(false)?;
        // NB: this necessary to free all the references to content data
        for snapshot_path in snapshot_paths.iter() {
            snapshot::delete_snapshot_file(snapshot_path)?;
        }
        fs::remove_dir(&self.dir_path)?;
        Ok(())
    }

    pub fn get_snapshot_paths(&self, reverse: bool) -> EResult<Vec<PathBuf>> {
        snapshot::get_snapshot_paths_in_dir(&self.dir_path, reverse)
    }

    pub fn get_snapshot_names(&self, reverse: bool) -> EResult<Vec<String>> {
        snapshot::get_snapshot_names_in_dir(&self.dir_path, reverse)
    }

    pub fn get_snapshot_path_back_n(&self, n: i64) -> EResult<PathBuf> {
        let snapshot_paths = self.get_snapshot_paths(true)?;
        if snapshot_paths.len() == 0 {
            return Err(Error::ArchiveEmpty(self.id.clone()));
        };
        let index: usize = if n < 0 {
            (snapshot_paths.len() as i64 + n) as usize
        } else {
            n as usize
        };
        if snapshot_paths.len() <= index {
            return Err(Error::SnapshotIndexOutOfRange(self.id.clone(), n));
        }
        Ok(snapshot_paths[index].clone())
    }

    pub fn delete_all_but_newest(&self, newest_count: usize, clear_fell: bool) -> EResult<usize> {
        let mut deleted_count: usize = 0;
        if !clear_fell && newest_count == 0 {
            return Err(Error::LastSnapshot(self.id.clone()));
        }
        let snapshot_paths = self.get_snapshot_paths(false)?;
        if snapshot_paths.len() == 0 {
            return Err(Error::ArchiveEmpty(self.id.clone()));
        }
        if snapshot_paths.len() <= newest_count {
            return Ok(0);
        }
        let last_index = snapshot_paths.len() - newest_count;
        for snapshot_path in snapshot_paths[0..last_index].iter() {
            snapshot::delete_snapshot_file(snapshot_path)?;
            deleted_count += 1;
        }
        Ok(deleted_count)
    }

    pub fn delete_ss_back_n(&self, n: i64, clear_fell: bool) -> EResult<usize> {
        let snapshot_paths = self.get_snapshot_paths(true)?;
        if snapshot_paths.len() == 0 {
            return Err(Error::ArchiveEmpty(self.id.clone()));
        };
        let index: usize = if n < 0 {
            (snapshot_paths.len() as i64 + n) as usize
        } else {
            n as usize
        };
        if snapshot_paths.len() <= index {
            return Ok(0);
        }
        if !clear_fell && snapshot_paths.len() == 1 {
            return Err(Error::LastSnapshot(self.id.clone()));
        }
        snapshot::delete_snapshot_file(&snapshot_paths[index])?;
        Ok(1)
    }

    pub fn copy_file_to(
        &self,
        n: i64,
        file_path: &Path,
        into_dir_path: &Path,
        opt_with_name: &Option<PathBuf>,
        overwrite: bool,
    ) -> EResult<(u64, time::Duration)> {
        let started_at = time::SystemTime::now();

        let snapshot_file_path = self.get_snapshot_path_back_n(n)?;
        let target_path = if let Some(with_name) = opt_with_name {
            into_dir_path.join(with_name)
        } else if let Some(file_name) = file_path.file_name() {
            into_dir_path.join(file_name)
        } else {
            panic!("{:?}: line {:?}", file!(), line!())
        };
        let abs_file_path = if file_path.starts_with("~") {
            pw_pathux::expand_home_dir(file_path).unwrap()
        } else {
            pw_pathux::absolute_path_buf(file_path)
        };
        let spd = SnapshotPersistentData::from_file(&snapshot_file_path)?;
        let bytes = spd.copy_file_to(&abs_file_path, &target_path, overwrite)?;

        let finished_at = time::SystemTime::now();
        let duration = match finished_at.duration_since(started_at) {
            Ok(duration) => duration,
            Err(_) => time::Duration::new(0, 0),
        };
        Ok((bytes, duration))
    }

    pub fn copy_dir_to(
        &self,
        n: i64,
        dir_path: &Path,
        into_dir_path: &Path,
        opt_with_name: &Option<PathBuf>,
        overwrite: bool,
    ) -> EResult<(ExtractionStats, time::Duration)> {
        let started_at = time::SystemTime::now();

        let snapshot_file_path = self.get_snapshot_path_back_n(n)?;
        let target_path = if let Some(with_name) = opt_with_name {
            into_dir_path.join(with_name)
        } else if let Some(dir_name) = dir_path.file_name() {
            into_dir_path.join(dir_name)
        } else {
            panic!("{:?}: line {:?}", file!(), line!())
        };
        let abs_dir_path = if dir_path.starts_with("~") {
            pw_pathux::expand_home_dir(dir_path).unwrap()
        } else {
            pw_pathux::absolute_path_buf(dir_path)
        };
        let spd = SnapshotPersistentData::from_file(&snapshot_file_path)?;
        let stats = spd.copy_dir_to(
            &abs_dir_path,
            &target_path,
            overwrite,
            &mut Some(&mut stderr()),
        )?;

        let finished_at = time::SystemTime::now();
        let duration = match finished_at.duration_since(started_at) {
            Ok(duration) => duration,
            Err(_) => time::Duration::new(0, 0),
        };
        Ok((stats, duration))
    }
}

#[cfg(test)]
mod tests {
    // TODO: fix tests to use temporary directories.
    use super::*;
    use std::env;

    #[test]
    fn test_file_exclusions() {
        let excl = Exclusions::new(&vec![], &vec!["*.[ao]".to_string(), "this.*".to_string()])
            .unwrap_or_else(|err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err));
        assert!(excl.is_excluded_file(&Path::new("whatever.o")));
        assert!(excl.is_excluded_file(&Path::new("whatever.a")));
        assert!(!excl.is_excluded_file(&Path::new("whatever.c")));
        assert!(!excl.is_excluded_file(&Path::new("whatevero")));
        assert!(!excl.is_excluded_file(&Path::new("whatevera")));
        assert!(excl.is_excluded_file(&Path::new("this.c")));
        assert!(excl.is_excluded_file(&Path::new("dir/whatever.o")));
        assert!(excl.is_excluded_file(&Path::new("dir/whatever.a")));
        assert!(!excl.is_excluded_file(&Path::new("dir/whatever.c")));
        assert!(!excl.is_excluded_file(&Path::new("dir/whatevero")));
        assert!(!excl.is_excluded_file(&Path::new("dir/whatevera")));
        assert!(excl.is_excluded_file(&Path::new("dir/this.c")));
    }

    #[test]
    fn test_dir_exclusions() {
        let excl = Exclusions::new(&vec!["*.[ao]".to_string(), "this.*".to_string()], &vec![])
            .unwrap_or_else(|err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err));
        assert!(excl.is_excluded_dir(&Path::new("whatever.o")));
        assert!(excl.is_excluded_dir(&Path::new("whatever.a")));
        assert!(!excl.is_excluded_dir(&Path::new("whatever.c")));
        assert!(!excl.is_excluded_dir(&Path::new("whatevero")));
        assert!(!excl.is_excluded_dir(&Path::new("whatevera")));
        assert!(excl.is_excluded_dir(&Path::new("this.c")));
        assert!(excl.is_excluded_dir(&Path::new("dir/whatever.o")));
        assert!(excl.is_excluded_dir(&Path::new("dir/whatever.a")));
        assert!(!excl.is_excluded_dir(&Path::new("dir/whatever.c")));
        assert!(!excl.is_excluded_dir(&Path::new("dir/whatevero")));
        assert!(!excl.is_excluded_dir(&Path::new("dir/whatevera")));
        assert!(excl.is_excluded_dir(&Path::new("dir/this.c")));
    }

    #[test]
    fn test_get_archive() {
        env::set_var("ERGIBUS_CONFIG_DIR", "../TEST/config");
        if let Err(err) = get_archive_data("dummy") {
            match err {
                Error::UnknownRepo(_) => (),
                _ => panic!("ERR: {:?}", err),
            }
        };
    }

    #[test]
    fn test_yaml_decode() {
        let yaml_str = "
content_repo_name: dummy\n
snapshot_dir_path: ./TEST/store/ergibus/archives/dummy\n
inclusions:\n
   - ~/SRC/GITHUB/ergibus.git/src\n
   - ~/SRC/GITHUB/ergibus.git/target\n
dir_exclusions:\n
   - lost+found\n
file_exclusions:\n
   - \"*.[oa]\"\n
   - \"*.py[co]\"\n
";
        let spec: ArchiveSpec = serde_yaml::from_str(&yaml_str)
            .unwrap_or_else(|err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err));
        assert_eq!(spec.content_repo_name, "dummy");
        assert_eq!(
            spec.snapshot_dir_path,
            "./TEST/store/ergibus/archives/dummy"
        );
        assert_eq!(
            spec.inclusions,
            vec![
                "~/SRC/GITHUB/ergibus.git/src",
                "~/SRC/GITHUB/ergibus.git/target"
            ]
        );
        assert_eq!(spec.dir_exclusions, vec!["lost+found"]);
        assert_eq!(spec.file_exclusions, vec!["*.[oa]", "*.py[co]"]);
    }

    #[test]
    fn test_read_write_archive_spec() {
        env::set_var("ERGIBUS_CONFIG_DIR", "../TEST/config");
        let spec: ArchiveSpec = read_archive_spec("dummy")
            .unwrap_or_else(|err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err));
        assert_eq!(spec.content_repo_name, "dummy");
        assert_eq!(
            spec.snapshot_dir_path,
            "./TEST/store/ergibus/archives/dummy"
        );
        assert_eq!(
            spec.inclusions,
            vec![
                "~/SRC/GITHUB/ergibus.git/src",
                "~/SRC/GITHUB/ergibus.git/target"
            ]
        );
        assert_eq!(spec.dir_exclusions, vec!["lost+found"]);
        assert_eq!(spec.file_exclusions, vec!["*.[oa]", "*.py[co]"]);
        if let Err(err) = write_archive_spec("dummy", &spec, true) {
            panic!("write spec failed: {:?}", err)
        };
        let spec: ArchiveSpec = read_archive_spec("dummy")
            .unwrap_or_else(|err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err));
        assert_eq!(spec.content_repo_name, "dummy");
        assert_eq!(
            spec.snapshot_dir_path,
            "./TEST/store/ergibus/archives/dummy"
        );
        assert_eq!(
            spec.inclusions,
            vec![
                "~/SRC/GITHUB/ergibus.git/src",
                "~/SRC/GITHUB/ergibus.git/target"
            ]
        );
        assert_eq!(spec.dir_exclusions, vec!["lost+found"]);
        assert_eq!(spec.file_exclusions, vec!["*.[oa]", "*.py[co]"]);
    }
}
