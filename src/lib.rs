#[macro_use]
extern crate serde_derive;

use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    str::FromStr,
};

use serde_yaml;

mod error;

pub use crate::error::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub enum HashAlgorithm {
    Sha1,
    Sha256,
    Sha512,
}

impl FromStr for HashAlgorithm {
    type Err = RepoError;
    fn from_str(src: &str) -> Result<HashAlgorithm, RepoError> {
        match src {
            "Sha1" | "SHA1" | "sha1" => Ok(HashAlgorithm::Sha1),
            "Sha256" | "SHA256" | "sha256" => Ok(HashAlgorithm::Sha256),
            "Sha512" | "SHA512" | "sha512" => Ok(HashAlgorithm::Sha512),
            _ => Err(RepoError::UnknownKeyAlgorithm(src.to_string())),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct RepoSpec {
    base_dir_path: PathBuf,
    hash_algorithm: HashAlgorithm,
}

impl RepoSpec {
    pub fn new<P: AsRef<Path>>(base_dir_path: P, hash_algorithm: HashAlgorithm) -> Self {
        let base_dir_path = base_dir_path.as_ref().to_path_buf();
        Self { base_dir_path, hash_algorithm }
    }

    pub fn from_reader(reader: impl Read) -> Result<Self, RepoError> {
        let spec: Self = serde_yaml::from_reader(reader)?;
        Ok(spec)
    }

    pub fn to_writer(&self, writer: impl Write) -> Result<(), RepoError> {
        serde_yaml::to_writer(writer, self)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use tempdir::TempDir;

    use super::*;

    #[test]
    fn repo_spec() {
        let repo_spec = RepoSpec::new("~/whatever", HashAlgorithm::Sha256);
        let tmp_dir = TempDir::new("TEST").unwrap();
        let path = tmp_dir.path().join("repo_spec");
        let file = File::create(&path).unwrap();
        repo_spec.to_writer(file).unwrap();
        let file = File::open(&path).unwrap();
        let read_repo_spec = RepoSpec::from_reader(file);
        assert_eq!(read_repo_spec.unwrap(), repo_spec);
    }
}
