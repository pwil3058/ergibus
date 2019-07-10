#[macro_use]
extern crate serde_derive;

use std::{
    cell::RefCell,
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    str::FromStr,
};

use crypto_hash;
use fs2::FileExt;
use hex::ToHex;
use serde_json;
use serde_yaml;
use snap;

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

impl HashAlgorithm {
    pub fn data_digest(&self, data: &[u8]) -> Result<String, io::Error> {
        let mut hasher = match self {
            HashAlgorithm::Sha1 => crypto_hash::Hasher::new(crypto_hash::Algorithm::SHA1),
            HashAlgorithm::Sha256 => crypto_hash::Hasher::new(crypto_hash::Algorithm::SHA256),
            HashAlgorithm::Sha512 => crypto_hash::Hasher::new(crypto_hash::Algorithm::SHA512),
        };
        hasher.write_all(data)?;
        let mut s = String::new();
        hasher.finish().write_hex_upper(&mut s).expect("HEX format failed");
        Ok(s)
    }

    pub fn reader_digest<R: Read>(&self, reader: &mut R) -> Result<String, io::Error> {
        let mut buffer = [0; 512000];
        let mut hasher = match self {
            HashAlgorithm::Sha1 => crypto_hash::Hasher::new(crypto_hash::Algorithm::SHA1),
            HashAlgorithm::Sha256 => crypto_hash::Hasher::new(crypto_hash::Algorithm::SHA256),
            HashAlgorithm::Sha512 => crypto_hash::Hasher::new(crypto_hash::Algorithm::SHA512),
        };
        loop {
            let n_bytes = reader.read(&mut buffer)?;
            if n_bytes == 0 {
                break;
            };
            hasher.write_all(&buffer[..n_bytes])?;
        }
        let mut s = String::new();
        hasher.finish().write_hex_upper(&mut s).expect("HEX format failed");
        Ok(s)
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

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct ContentMgmtKey {
    base_dir_path: PathBuf,
    ref_counter_path: PathBuf,
    hash_algortithm: HashAlgorithm,
}

impl From<&RepoSpec> for ContentMgmtKey {
    fn from(spec: &RepoSpec) -> ContentMgmtKey {
        let base_dir_path = PathBuf::from(&spec.base_dir_path);
        ContentMgmtKey {
            ref_counter_path: base_dir_path.join("ref_count"),
            base_dir_path: base_dir_path,
            hash_algortithm: spec.hash_algorithm,
        }
    }
}

impl ContentMgmtKey {
    pub fn open_content_manager(&self, mutable: bool) -> Result<ContentManager, RepoError> {
        let mut hash_map_file = self.locked_ref_count_file(mutable)?;
        let ref_counter = ProtectedRefCounter::from_file(&mut hash_map_file, mutable)?;
        Ok(ContentManager{
            content_mgmt_key: self.clone(),
            ref_counter: ref_counter,
            hash_map_file: hash_map_file
        })
    }

    fn locked_ref_count_file(&self, mutable: bool) -> Result<File, RepoError> {
        let file = OpenOptions::new()
                    .read(true)
                    .write(mutable)
                    .open(&self.ref_counter_path)?;
        if mutable {
            file.lock_exclusive()?;
        } else {
            file.lock_shared()?;
        }
        Ok(file)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub struct RefCountData {
    ref_count: u64,
    content_size: u64,
    stored_size: u64
}

#[derive(Serialize, Deserialize, Debug)]
struct RefCounter (HashMap<String, RefCountData>);

impl RefCounter {
    fn from_file(file: &mut File) -> Result<RefCounter, RepoError> {
        let mut rchp_str = String::new();
        let mut snappy_rdr = snap::Reader::new(file);
        snappy_rdr.read_to_string(&mut rchp_str)?;
        let rchp = serde_json::from_str::<RefCounter>(&rchp_str)?;
        Ok(rchp)
    }

    fn _to_file(&self, file: &mut File) -> Result<(), RepoError> {
        let json_text = serde_json::to_string(self)?;
        file.seek(SeekFrom::Start(0))?;
        file.set_len(0)?;
        let mut snappy_wtr = snap::Writer::new(file);
        snappy_wtr.write_all(json_text.as_bytes())?;
        Ok(())
    }
}

#[derive(Debug)]
enum ProtectedRefCounter {
    Immutable(RefCounter),
    Mutable(RefCell<RefCounter>)
}

impl ProtectedRefCounter { // GENERAL
    fn _is_mutable(&self) -> bool {
        match *self {
            ProtectedRefCounter::Immutable(_) => false,
            ProtectedRefCounter::Mutable(_) => true
        }
    }

    fn from_file(file: &mut File, mutable: bool) -> Result<ProtectedRefCounter, RepoError> {
        let ref_counter = RefCounter::from_file(file)?;
        if mutable {
            Ok(ProtectedRefCounter::Mutable(RefCell::new(ref_counter)))
        } else {
            Ok(ProtectedRefCounter::Immutable(ref_counter))
        }
    }
}

#[derive(Debug)]
pub struct ContentManager {
    content_mgmt_key: ContentMgmtKey,
    ref_counter: ProtectedRefCounter,
    hash_map_file: File
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
        repo_spec.to_writer(&file).unwrap();
        let file = File::open(&path).unwrap();
        let read_repo_spec = RepoSpec::from_reader(&file);
        assert_eq!(read_repo_spec.unwrap(), repo_spec);
    }

    #[test]
    fn content_digest() {
        let hash_algorithm = HashAlgorithm::Sha1;
        let mut data: Vec<u8> = vec![0, 1, 2, 3, 4];
        assert_eq!(
            hash_algorithm.data_digest(&mut data).unwrap(),
            "1CF251472D59F8FADEB3AB258E90999D8491BE19".to_string()
        )
    }
}
