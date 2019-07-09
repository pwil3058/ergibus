#[macro_use]
extern crate serde_derive;

use std::{
    io::{Read, Write},
    path::PathBuf,
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
    pub fn from_reader(reader: impl Read) -> Result<Self, RepoError> {
        let spec: RepoSpec = serde_yaml::from_reader(reader)?;
        Ok(spec)
    }

    pub fn to_writer(&self, writer: impl Write) -> Result<(), RepoError> {
        serde_yaml::to_writer(writer, self)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
