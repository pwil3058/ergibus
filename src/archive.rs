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

use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::path::{Path, PathBuf};

use globset::{self, Glob, GlobSet, GlobSetBuilder};
use serde_yaml;

use content::{ContentMgmtKey, get_content_mgmt_key};
use eerror::{AError, CError};
use pathux::{expand_home_dir};

#[derive(Debug)]
pub struct Exclusions {
    dir_globset: GlobSet,
    file_globset: GlobSet
}

impl Exclusions {
    fn new(dir_patterns: &Vec<String>, file_patterns: &Vec<String>) -> Result<Exclusions, AError> {
        let mut dgs_builder = GlobSetBuilder::new();
        for pattern in dir_patterns {
            let glob = Glob::new(pattern).map_err(|err| AError::GlobError(err))?;
            dgs_builder.add(glob);
        }
        let dir_globset = dgs_builder.build().map_err(|err| AError::GlobError(err))?;

        let mut fgs_builder = GlobSetBuilder::new();
        for pattern in file_patterns {
            let glob = Glob::new(pattern).map_err(|err| AError::GlobError(err))?;
            fgs_builder.add(glob);
        }
        let file_globset = fgs_builder.build().map_err(|err| AError::GlobError(err))?;

        Ok(Exclusions{dir_globset, file_globset})
    }

    pub fn is_excluded_dir(&self, abs_dir_path: &Path) -> bool {
        if self.dir_globset.is_empty() {
            return false;
        } else if self.dir_globset.is_match(abs_dir_path) {
            return true;
        } else {
            let dir_name = abs_dir_path.file_name().unwrap();
            return self.dir_globset.is_match(&dir_name);
        }
    }

    pub fn is_excluded_file(&self, abs_file_path: &Path) -> bool {
        if self.file_globset.is_empty() {
            return false;
        } else if self.file_globset.is_match(abs_file_path) {
            return true;
        } else {
            let file_name = abs_file_path.file_name().unwrap();
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

fn read_archive_spec(archive_name: &str) -> Result<ArchiveSpec, AError> {
    let config_dir_path = Path::new("./TEST/config/archives").canonicalize().unwrap();
    let mut spec_file_path = config_dir_path.join(archive_name);
    spec_file_path.set_extension("aspec");
    let mut spec_file = File::open(&spec_file_path).map_err(|err| AError::IOError(err, spec_file_path.clone()))?;
    let spec: ArchiveSpec = serde_yaml::from_reader(&spec_file).map_err(|err| AError::YamlError(err))?;
    Ok(spec)
}

fn write_archive_spec(archive_name: &str, archive_spec: &ArchiveSpec) -> Result<(), AError> {
    let config_dir_path = Path::new("./TEST/config/archives").canonicalize().unwrap();
    let mut spec_file_path = config_dir_path.join(archive_name);
    spec_file_path.set_extension("aspec");
    let mut spec_file = File::create(&spec_file_path).map_err(|err| AError::IOError(err, spec_file_path.clone()))?;
    serde_yaml::to_writer(&spec_file, archive_spec).map_err(|err| AError::YamlError(err))?;
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

pub fn get_archive_data(archive_name: &str) -> Result<ArchiveData, AError> {
    let archive_spec = read_archive_spec(archive_name)?;
    let name = archive_name.to_string();
    let content_mgmt_key = get_content_mgmt_key(&archive_spec.content_repo_name).map_err(|err| AError::ContentError(err))?;
    let snapshot_dir_path = PathBuf::from(&archive_spec.snapshot_dir_path).canonicalize().map_err(|err| AError::IOError(err, PathBuf::from(&archive_spec.snapshot_dir_path)))?;
    let mut includes = Vec::new();
    for inclusion in archive_spec.inclusions {
        let included_file_path = if inclusion.starts_with("~") {
            expand_home_dir(&PathBuf::from(inclusion))
        } else {
            let path_buf = PathBuf::from(inclusion);
            if path_buf.is_relative() {
                return Err(AError::RelativeIncludePath(path_buf));
            };
            path_buf
        };
        includes.push(included_file_path);
    }
    let exclusions = Exclusions::new(&archive_spec.dir_exclusions, &archive_spec.file_exclusions)?;

    Ok(ArchiveData{name, content_mgmt_key, snapshot_dir_path, includes, exclusions,})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_exclusions() {
        let excl = Exclusions::new(&vec![], &vec!["*.[ao]".to_string(), "this.*".to_string()]).unwrap();
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
        let excl = Exclusions::new(&vec!["*.[ao]".to_string(), "this.*".to_string()], &vec![]).unwrap();
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
        let archive = get_archive_data("dummy");
        assert!(archive.is_ok());
        //assert_eq!("dummy".to_string(), archive.name);
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
        let spec: ArchiveSpec = serde_yaml::from_str(&yaml_str).unwrap();
        assert_eq!(spec.content_repo_name, "dummy");
        assert_eq!(spec.snapshot_dir_path, "./TEST/store/ergibus/archives/dummy");
        assert_eq!(spec.inclusions, vec!["~/SRC/GITHUB/ergibus.git/src", "~/SRC/GITHUB/ergibus.git/target"]);
        assert_eq!(spec.dir_exclusions, vec!["lost+found"]);
        assert_eq!(spec.file_exclusions, vec!["*.[oa]", "*.py[co]"]);
    }

    #[test]
    fn test_read_write_archive_spec() {
        let spec: ArchiveSpec = read_archive_spec("dummy").unwrap();
        assert_eq!(spec.content_repo_name, "dummy");
        assert_eq!(spec.snapshot_dir_path, "./TEST/store/ergibus/archives/dummy");
        assert_eq!(spec.inclusions, vec!["~/SRC/GITHUB/ergibus.git/src", "~/SRC/GITHUB/ergibus.git/target"]);
        assert_eq!(spec.dir_exclusions, vec!["lost+found"]);
        assert_eq!(spec.file_exclusions, vec!["*.[oa]", "*.py[co]"]);
        if let Err(err) = write_archive_spec("dummy", &spec) {
            panic!("write spec failed")
        };
        let spec: ArchiveSpec = read_archive_spec("dummy").unwrap();
        assert_eq!(spec.content_repo_name, "dummy");
        assert_eq!(spec.snapshot_dir_path, "./TEST/store/ergibus/archives/dummy");
        assert_eq!(spec.inclusions, vec!["~/SRC/GITHUB/ergibus.git/src", "~/SRC/GITHUB/ergibus.git/target"]);
        assert_eq!(spec.dir_exclusions, vec!["lost+found"]);
        assert_eq!(spec.file_exclusions, vec!["*.[oa]", "*.py[co]"]);
    }
}
