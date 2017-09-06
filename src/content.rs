use std::cell::Cell;
use std::fs::{self, File};
use std::io::prelude::*;
use std::io;
use std::path::{Path, PathBuf};

use hex::ToHex;

use crypto;
use crypto::digest::Digest;

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
    Sha2,
    Sha3,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct ContentMgmtKey {
    base_dir_path: PathBuf,
    ref_counter_path: PathBuf,
    lock_file_path: PathBuf,
    hash_algortith: HashAlgorithm,
    compressed: bool,
}

impl ContentMgmtKey {
    pub fn new_dummy() -> ContentMgmtKey {
        ContentMgmtKey {
            base_dir_path: PathBuf::from("whatever"),
            ref_counter_path: PathBuf::from("whatever"),
            lock_file_path: PathBuf::from("whatever"),
            hash_algortith: HashAlgorithm::Sha1,
            compressed: true,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ContentManager {
    count: Cell<i64>,
}

impl Drop for ContentManager {
    fn drop(&mut self) {
        // TODO: write json to file if we were open for writing
    }
}

const use_crypto_hash: bool = true;

fn file_digest(file: &mut File) -> Result<String, io::Error> {
    let mut buffer = [0; 1000000];
    if use_crypto_hash {
        let mut hasher = crypto_hash::Hasher::new(crypto_hash::Algorithm::SHA1);
        loop {
            let n_bytes = file.read(&mut buffer)?;
            if n_bytes == 0 {
                break;
            };
            hasher.write_all(&buffer[..n_bytes]);
        }
        Ok(hasher.finish().to_hex())
    } else {
        let mut hasher = crypto::sha1::Sha1::new();
        loop {
            let n_bytes = file.read(&mut buffer)?;
            if n_bytes == 0 {
                break;
            };
            hasher.input(&buffer[..n_bytes]);
        }
        Ok(hasher.result_str())
    }
}

impl ContentManager {
    pub fn new(content_mgmt_key: &ContentMgmtKey, for_write: bool) -> ContentManager {
        ContentManager{count: Cell::new(0)}
    }

    pub fn store_file_contents(&self, abs_file_path: &Path) -> Result<String, CError> {
        self.count.replace(self.count.get() + 1);
        let mut file = File::open(abs_file_path).map_err(|err| CError::FileSystemError(err))?;
        let digest = file_digest(&mut file).map_err(|err| CError::FileSystemError(err))?;
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
