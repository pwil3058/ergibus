use std::fs::{self, File};
use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};
use hostname;
use serde_yaml;
use users;

use config;
use content::{ContentMgmtKey, get_content_mgmt_key, content_repo_exists};
use eerror::{EError, EResult};
use pw_pathux::{expand_home_dir};

#[derive(Debug)]
pub struct Exclusions {
    dir_globset: GlobSet,
    file_globset: GlobSet
}

impl Exclusions {
    fn new(dir_patterns: &Vec<String>, file_patterns: &Vec<String>) -> EResult<Exclusions> {
        let mut dgs_builder = GlobSetBuilder::new();
        for pattern in dir_patterns {
            let glob = Glob::new(pattern).map_err(|err| EError::GlobError(err))?;
            dgs_builder.add(glob);
        }
        let dir_globset = dgs_builder.build().map_err(|err| EError::GlobError(err))?;

        let mut fgs_builder = GlobSetBuilder::new();
        for pattern in file_patterns {
            let glob = Glob::new(pattern).map_err(|err| EError::GlobError(err))?;
            fgs_builder.add(glob);
        }
        let file_globset = fgs_builder.build().map_err(|err| EError::GlobError(err))?;

        Ok(Exclusions{dir_globset, file_globset})
    }

    pub fn is_excluded_dir(&self, abs_dir_path: &Path) -> bool {
        if self.dir_globset.is_empty() {
            return false;
        } else if self.dir_globset.is_match(abs_dir_path) {
            return true;
        } else {
            let dir_name = match abs_dir_path.file_name() {
                Some(dir_name) => dir_name,
                None => panic!("{:?}: line {:?}", file!(), line!())
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
                None => panic!("{:?}: line {:?}", file!(), line!())
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
    file_exclusions: Vec<String>
}

fn get_archive_spec_file_path(archive_name: &str) -> PathBuf {
    config::get_archive_config_dir_path().join(archive_name)
}

fn read_archive_spec(archive_name: &str) -> EResult<ArchiveSpec> {
    let spec_file_path = get_archive_spec_file_path(archive_name);
    // TODO: map error to a more specific EError.
    let spec_file = File::open(&spec_file_path).map_err(|err| EError::ArchiveReadError(err, spec_file_path.clone()))?;
    let spec: ArchiveSpec = serde_yaml::from_reader(&spec_file).map_err(|err| EError::ArchiveYamlReadError(err, archive_name.to_string()))?;
    Ok(spec)
}

fn write_archive_spec(archive_name: &str, archive_spec: &ArchiveSpec, overwrite: bool) -> EResult<()> {
    let spec_file_path = get_archive_spec_file_path(archive_name);
    if !overwrite && spec_file_path.exists() {
        return Err(EError::ArchiveExists(archive_name.to_string()))
    }
    match spec_file_path.parent() {
        Some(config_dir_path) => if !config_dir_path.exists() {
            fs::create_dir_all(&config_dir_path).map_err(|err| EError::ArchiveWriteError(err, config_dir_path.to_path_buf()))?;
        },
        None => (),
    }
    let spec_file = File::create(&spec_file_path).map_err(|err| EError::ArchiveWriteError(err, spec_file_path.clone()))?;
    serde_yaml::to_writer(&spec_file, archive_spec).map_err(|err| EError::ArchiveYamlWriteError(err, archive_name.to_string()))?;
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

pub fn create_new_archive(name: &str, content_repo_name: &str, location: &str, inclusions: Vec<String>, dir_exclusions: Vec<String>, file_exclusions: Vec<String>) -> EResult<()> {
    if get_archive_spec_file_path(name).exists() {
        return Err(EError::ArchiveExists(name.to_string()))
    }
    if !content_repo_exists(content_repo_name) {
        return Err(EError::UnknownRepo(content_repo_name.to_string()))
    }
    for pattern in &dir_exclusions {
        let _glob = Glob::new(&pattern).map_err(|err| EError::GlobError(err))?;
    }
    for pattern in &file_exclusions {
        let _glob = Glob::new(&pattern).map_err(|err| EError::GlobError(err))?;
    }
    let mut snapshot_dir_path = PathBuf::from(location);
    snapshot_dir_path.push("ergibus");
    snapshot_dir_path.push("archives");
    match hostname::get_hostname() {
        Some(hostname) => snapshot_dir_path.push(hostname),
        None => ()
    };
    match users::get_current_username() {
        Some(user_name) => snapshot_dir_path.push(user_name),
        None => ()
    };
    snapshot_dir_path.push(name);
    fs::create_dir_all(&snapshot_dir_path).map_err(|err| EError::ArchiveWriteError(err, snapshot_dir_path.clone()))?;
    let sdp_str = match snapshot_dir_path.to_str() {
        Some(sdp_ostr) => sdp_ostr.to_string(),
        None => panic!("{:?}: line {:?}", file!(), line!())
    };
    let spec = ArchiveSpec {
        content_repo_name: content_repo_name.to_string(),
        snapshot_dir_path: sdp_str,
        inclusions: inclusions,
        dir_exclusions: dir_exclusions,
        file_exclusions: file_exclusions
    };
    write_archive_spec(name, &spec, false)?;
    Ok(())
}

pub fn get_archive_data(archive_name: &str) -> EResult<ArchiveData> {
    let archive_spec = read_archive_spec(archive_name)?;
    let name = archive_name.to_string();
    let content_mgmt_key = get_content_mgmt_key(&archive_spec.content_repo_name)?;
    let snapshot_dir_path = PathBuf::from(&archive_spec.snapshot_dir_path).canonicalize().map_err(|err| EError::ArchiveDirError(err, PathBuf::from(&archive_spec.snapshot_dir_path)))?;
    let mut includes = Vec::new();
    for inclusion in archive_spec.inclusions {
        let included_file_path = if inclusion.starts_with("~") {
            match expand_home_dir(&PathBuf::from(inclusion)) {
                Some(expanded_path) => expanded_path,
                None => panic!("{:?}: line {:?}: home dir expansion failed", file!(), line!())
            }
        } else {
            let path_buf = PathBuf::from(inclusion);
            if path_buf.is_relative() {
                return Err(EError::RelativeIncludePath(path_buf, archive_name.to_string()));
            };
            path_buf
        };
        includes.push(included_file_path);
    }
    let exclusions = Exclusions::new(&archive_spec.dir_exclusions, &archive_spec.file_exclusions)?;

    Ok(ArchiveData{name, content_mgmt_key, snapshot_dir_path, includes, exclusions,})
}

// for read only snapshot actions we only need the snapshot directory path
// as the content manager key data is in the snapshot file.
// NB: this means that we can use snapshots even if the configuration
// data has been lost due to a file system failure (but in that case
// the user will have to browse the file system to find the snapshots).
pub fn get_archive_snapshot_dir_path(archive_name: &str)  -> EResult<PathBuf> {
    let archive_spec = read_archive_spec(archive_name)?;
    PathBuf::from(&archive_spec.snapshot_dir_path).canonicalize().map_err(|err| EError::ArchiveDirError(err, PathBuf::from(&archive_spec.snapshot_dir_path)))
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

#[cfg(test)]
mod tests {
    use std::env;
    use super::*;

    #[test]
    fn test_file_exclusions() {
        let excl = Exclusions::new(&vec![], &vec!["*.[ao]".to_string(), "this.*".to_string()]).unwrap_or_else(
            |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
        );
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
        let excl = Exclusions::new(&vec!["*.[ao]".to_string(), "this.*".to_string()], &vec![]).unwrap_or_else(
            |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
        );
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
        env::set_var("ERGIBUS_CONFIG_DIR", "./TEST/config");
        if let Err(err) = get_archive_data("dummy") {
            match err {
                EError::UnknownRepo(_) => (),
                _ => panic!("ERR: {:?}", err)
            }
        };
    }

    #[test]
    fn test_yaml_decode() {
        let yaml_str =
"
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
        let spec: ArchiveSpec = serde_yaml::from_str(&yaml_str).unwrap_or_else(
            |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
        );
        assert_eq!(spec.content_repo_name, "dummy");
        assert_eq!(spec.snapshot_dir_path, "./TEST/store/ergibus/archives/dummy");
        assert_eq!(spec.inclusions, vec!["~/SRC/GITHUB/ergibus.git/src", "~/SRC/GITHUB/ergibus.git/target"]);
        assert_eq!(spec.dir_exclusions, vec!["lost+found"]);
        assert_eq!(spec.file_exclusions, vec!["*.[oa]", "*.py[co]"]);
    }

    #[test]
    fn test_read_write_archive_spec() {
        env::set_var("ERGIBUS_CONFIG_DIR", "./TEST/config");
        let spec: ArchiveSpec = read_archive_spec("dummy").unwrap_or_else(
            |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
        );
        assert_eq!(spec.content_repo_name, "dummy");
        assert_eq!(spec.snapshot_dir_path, "./TEST/store/ergibus/archives/dummy");
        assert_eq!(spec.inclusions, vec!["~/SRC/GITHUB/ergibus.git/src", "~/SRC/GITHUB/ergibus.git/target"]);
        assert_eq!(spec.dir_exclusions, vec!["lost+found"]);
        assert_eq!(spec.file_exclusions, vec!["*.[oa]", "*.py[co]"]);
        if let Err(err) = write_archive_spec("dummy", &spec, true) {
            panic!("write spec failed: {:?}", err)
        };
        let spec: ArchiveSpec = read_archive_spec("dummy").unwrap_or_else(
            |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
        );
        assert_eq!(spec.content_repo_name, "dummy");
        assert_eq!(spec.snapshot_dir_path, "./TEST/store/ergibus/archives/dummy");
        assert_eq!(spec.inclusions, vec!["~/SRC/GITHUB/ergibus.git/src", "~/SRC/GITHUB/ergibus.git/target"]);
        assert_eq!(spec.dir_exclusions, vec!["lost+found"]);
        assert_eq!(spec.file_exclusions, vec!["*.[oa]", "*.py[co]"]);
    }
}
