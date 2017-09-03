use std::path::{Path};

use globset::{self, Glob, GlobSet, GlobSetBuilder};

#[derive(Debug)]
pub enum AError {
    GlobError(globset::Error),
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
}
