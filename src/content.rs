use std::cell::Cell;
use std::io;
use std::path::{Path, PathBuf};

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

#[derive(Debug)]
pub enum ContentError {
    UnknownToken(String),
    FileSystemError(io::Error),
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

impl ContentManager {
    pub fn new(content_mgmt_key: &ContentMgmtKey, for_write: bool) -> ContentManager {
        ContentManager{count: Cell::new(0)}
    }

    pub fn store_file_contents(&self, abs_file_path: &Path) -> Result<String, ContentError> {
        self.count.replace(self.count.get() + 1);
        Ok(format!("Token Key: {:?}", self.count.get()))
    }

    pub fn release_contents(&self, content_token: &str) -> Result<(), ContentError> {
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
