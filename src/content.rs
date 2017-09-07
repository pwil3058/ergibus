use std::cell::Cell;
use std::fs::{self, File};
use std::io::prelude::*;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use hex::ToHex;

use crypto_hash;

pub fn get_content_mgmt_key(repo_name: &str) -> Result<ContentMgmtKey, CError> {
    if repo_name == "dummy" {
        Ok(ContentMgmtKey::new_dummy())
    } else {
        Err(CError::UnknownRepo(repo_name.to_string()))
    }
}

#[derive(Debug)]
pub enum CError {
    UnknownRepo(String),
    IOError(io::Error, PathBuf),
    UnknownToken(String),
    FileSystemError(io::Error),
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub enum HashAlgorithm {
    Sha1,
    Sha256,
    Sha512,
}

impl FromStr for HashAlgorithm {
    type Err = ();
    fn from_str(src: &str) -> Result<HashAlgorithm, ()> {
        return match src {
            "Sha1" | "SHA1" | "sha1" => Ok(HashAlgorithm::Sha1),
            "Sha256" | "SHA256" | "sha256" => Ok(HashAlgorithm::Sha256),
            "Sha512" | "SHA512" | "sha512" => Ok(HashAlgorithm::Sha512),
            _ => Err(()),
        };
    }
}


#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct ContentMgmtKey {
    base_dir_path: PathBuf,
    ref_counter_path: PathBuf,
    lock_file_path: PathBuf,
    hash_algortithm: HashAlgorithm,
    compressed: bool,
}

impl ContentMgmtKey {
    pub fn new_dummy() -> ContentMgmtKey {
        ContentMgmtKey {
            base_dir_path: PathBuf::from("whatever"),
            ref_counter_path: PathBuf::from("whatever"),
            lock_file_path: PathBuf::from("whatever"),
            hash_algortithm: HashAlgorithm::Sha1,
            compressed: true,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ContentManager {
    content_mgmt_key: ContentMgmtKey,
    count: Cell<i64>,
}

impl Drop for ContentManager {
    fn drop(&mut self) {
        // TODO: write json to file if we were open for writing
    }
}

fn file_digest(hash_algorithm: HashAlgorithm, file: &mut File) -> Result<String, io::Error> {
    let mut buffer = [0; 512000];
    let mut hasher = match hash_algorithm {
        Sha1 => crypto_hash::Hasher::new(crypto_hash::Algorithm::SHA1),
        Sha256 => crypto_hash::Hasher::new(crypto_hash::Algorithm::SHA256),
        Sha512 => crypto_hash::Hasher::new(crypto_hash::Algorithm::SHA512),
    };
    loop {
        let n_bytes = file.read(&mut buffer)?;
        if n_bytes == 0 {
            break;
        };
        hasher.write_all(&buffer[..n_bytes]);
    }
    Ok(hasher.finish().to_hex())
}

impl ContentManager {
    pub fn new(content_mgmt_key: &ContentMgmtKey, for_write: bool) -> ContentManager {
        ContentManager{count: Cell::new(0), content_mgmt_key: content_mgmt_key.clone()}
    }

    pub fn store_file_contents(&self, abs_file_path: &Path) -> Result<String, CError> {
        self.count.replace(self.count.get() + 1);
        let mut file = File::open(abs_file_path).map_err(|err| CError::FileSystemError(err))?;
        let digest = file_digest(self.content_mgmt_key.hash_algortithm, &mut file).map_err(|err| CError::FileSystemError(err))?;
        Ok(digest)
    }

    pub fn release_contents(&self, content_token: &str) -> Result<(), CError> {
        self.count.replace(self.count.get() - 1);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
