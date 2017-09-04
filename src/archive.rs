use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::path::{Path, PathBuf};

use globset::{self, Glob, GlobSet, GlobSetBuilder};

use content::{ContentMgmtKey, CError, get_content_mgmt_key};
use pathux::{expand_home_dir};

#[derive(Debug)]
pub enum AError {
    GlobError(globset::Error),
    IOError(io::Error, PathBuf),
    RelativeIncludePath(PathBuf),
    ContentError(CError),
}

#[derive(Debug)]
pub struct Exclusions {
    dir_globset: GlobSet,
    file_globset: GlobSet
}

impl Exclusions {
    pub fn new_dummy() -> Result<Exclusions, AError> {
        Exclusions::new(&vec!["lost+found", "TEST"], &vec!["*.o", "*.a"])
    }

    pub fn new(dir_patterns: &Vec<&str>, file_patterns: &Vec<&str>) -> Result<Exclusions, AError> {
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

#[derive(Debug)]
pub struct ArchiveData {
    pub name: String,
    pub content_mgmt_key: ContentMgmtKey,
    pub snapshot_dir_path: PathBuf,
    pub includes: Vec<PathBuf>,
    pub exclusions: Exclusions,
}

pub fn get_archive_data(archive_name: &str) -> Result<ArchiveData, AError> {
    let config_dir_path = Path::new("./TEST/config/archives").canonicalize().unwrap();
    let spec_dir_path = config_dir_path.join(archive_name);

    let name = archive_name.to_string();

    let spec_file_path = spec_dir_path.join("spec");
    let mut spec_file = File::open(&spec_file_path).map_err(|err| AError::IOError(err, spec_file_path.clone()))?;
    let mut buffer = String::new();
    spec_file.read_to_string(&mut buffer).map_err(|err| AError::IOError(err, spec_file_path.clone()))?;
    let lines: Vec<&str> = buffer.lines().collect();
    assert!(lines.len() == 2);
    let content_mgmt_key = get_content_mgmt_key(lines[0]).map_err(|err| AError::ContentError(err))?;
    let snapshot_dir_path = PathBuf::from(lines[1]).canonicalize().map_err(|err| AError::IOError(err, PathBuf::from(lines[1])))?;

    let includes_file_path = spec_dir_path.join("includes");
    let mut includes_file = File::open(&includes_file_path).map_err(|err| AError::IOError(err, includes_file_path.clone()))?;
    let mut buffer = String::new();
    includes_file.read_to_string(&mut buffer).map_err(|err| AError::IOError(err, includes_file_path.clone()))?;
    let mut includes = Vec::new();
    for line in buffer.lines() {
        let included_file_path = if line.starts_with("~") {
            expand_home_dir(&PathBuf::from(line))
        } else {
            let path_buf = PathBuf::from(line);
            if path_buf.is_relative() {
                return Err(AError::RelativeIncludePath(path_buf));
            };
            path_buf
        };
        includes.push(included_file_path);
    }

    let exclude_dirs_file_path = spec_dir_path.join("exclude_dirs");
    let mut exclude_dirs_file = File::open(&exclude_dirs_file_path).map_err(|err| AError::IOError(err, exclude_dirs_file_path.clone()))?;
    let mut buffer = String::new();
    exclude_dirs_file.read_to_string(&mut buffer).map_err(|err| AError::IOError(err, exclude_dirs_file_path.clone()))?;
    let dir_patterns: Vec<&str> = buffer.lines().collect();

    let exclude_files_file_path = spec_dir_path.join("exclude_files");
    let mut exclude_files_file = File::open(&exclude_files_file_path).map_err(|err| AError::IOError(err, exclude_files_file_path.clone()))?;
    let mut buffer = String::new();
    exclude_files_file.read_to_string(&mut buffer).map_err(|err| AError::IOError(err, exclude_files_file_path.clone()))?;
    let file_patterns: Vec<&str> = buffer.lines().collect();

    let exclusions = Exclusions::new(&dir_patterns, &file_patterns)?;

    Ok(ArchiveData { name, content_mgmt_key, snapshot_dir_path, includes, exclusions,})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_exclusions() {
        let excl = Exclusions::new(&vec![], &vec!["*.[ao]", "this.*"]).unwrap();
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
        let excl = Exclusions::new(&vec!["*.[ao]", "this.*"], &vec![]).unwrap();
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
}
