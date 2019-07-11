#[macro_use]
extern crate serde_derive;

use std::{
    cell::RefCell,
    collections::HashMap,
    fs::{create_dir_all, remove_file, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    str::FromStr,
};

use crypto_hash;
use fs2::FileExt;
use hex::ToHex;
use serde::{Deserialize, Serialize};
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
            _ => Err(RepoError::UnknownHashAlgorithm(src.to_string())),
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
        hasher
            .finish()
            .write_hex_upper(&mut s)
            .expect("HEX format failed");
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
        hasher
            .finish()
            .write_hex_upper(&mut s)
            .expect("HEX format failed");
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
        Self {
            base_dir_path,
            hash_algorithm,
        }
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

pub trait ContentManagerIfce: Drop + Sized {
    fn is_mutable(&self) -> bool;
    //fn key<'a, K: ContentMgmtKeyIfce<'a, Self>>(&'a self) -> &'a K;
    /// Non mutating methods
    fn check_content_token<R: Read>(&self, reader: &mut R, token: &str) -> Result<bool, RepoError>;
    fn ref_count_for_token(&self, token: &str) -> Result<u64, RepoError>;
    fn write_contents_for_token<W: Write>(
        &self,
        content_token: &str,
        writer: &mut W,
    ) -> Result<u64, RepoError>;

    /// Mutating methods: will cause a panic if called on immutable manager
    fn prune_contents(&self) -> Result<(u64, u64, u64), RepoError>;
    fn release_contents(&self, content_token: &str) -> Result<RefCountData, RepoError>;
    fn store_contents(&self, file: &mut File) -> Result<(String, u64, u64), RepoError>;
}

pub trait ContentMgmtKeyIfce<'a, M>
where
    Self: 'a + Sized + Serialize + Deserialize<'a> + PartialEq + Clone,
    Self: From<&'static RepoSpec>,
    M: ContentManagerIfce,
{
    fn open_content_manager(&self, mutable: bool) -> Result<M, RepoError>;
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
    pub fn create_repo_dir(&self) -> Result<(), RepoError> {
        if self.base_dir_path.exists() {
            return Err(RepoError::RepoDirExists(self.base_dir_path.clone()));
        }
        create_dir_all(&self.base_dir_path)?;
        let mut file = File::create(&self.ref_counter_path)?;
        RefCounter::new().to_file(&mut file)?;
        Ok(())
    }

    pub fn open_content_manager(&self, mutable: bool) -> Result<ContentManager, RepoError> {
        let mut hash_map_file = self.locked_ref_count_file(mutable)?;
        let ref_counter = ProtectedRefCounter::from_file(&mut hash_map_file, mutable)?;
        Ok(ContentManager {
            content_mgmt_key: self.clone(),
            ref_counter: ref_counter,
            hash_map_file: hash_map_file,
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

    fn token_content_file_path(&self, token: &str) -> PathBuf {
        let mut path_buf = self.base_dir_path.clone();
        path_buf.push(PathBuf::from(&token[0..3]));
        path_buf.push(PathBuf::from(&token[3..]));

        path_buf
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub struct RefCountData {
    ref_count: u64,
    content_size: u64,
    stored_size: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct RefCounter(HashMap<String, RefCountData>);

impl RefCounter {
    fn new() -> Self {
        Self { 0: HashMap::new() }
    }

    fn from_file(file: &mut File) -> Result<RefCounter, RepoError> {
        let mut rchp_str = String::new();
        let mut snappy_rdr = snap::Reader::new(file);
        snappy_rdr.read_to_string(&mut rchp_str)?;
        let rchp = serde_json::from_str::<RefCounter>(&rchp_str)?;
        Ok(rchp)
    }

    fn to_file(&self, file: &mut File) -> Result<(), RepoError> {
        let json_text = serde_json::to_string(self)?;
        file.seek(SeekFrom::Start(0))?;
        file.set_len(0)?;
        let mut snappy_wtr = snap::Writer::new(file);
        snappy_wtr.write_all(json_text.as_bytes())?;
        Ok(())
    }

    fn expired_tokens(&self) -> Vec<(String, RefCountData)> {
        self.0
            .iter()
            .filter(|(_, rcd)| rcd.ref_count == 0)
            .map(|(t, rcd)| (t.clone(), *rcd))
            .collect()
    }
}

#[derive(Debug)]
enum ProtectedRefCounter {
    Immutable(RefCounter),
    Mutable(RefCell<RefCounter>),
}

impl ProtectedRefCounter {
    // GENERAL
    fn is_mutable(&self) -> bool {
        match *self {
            ProtectedRefCounter::Immutable(_) => false,
            ProtectedRefCounter::Mutable(_) => true,
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

impl ProtectedRefCounter {
    // MUTABLE
    fn to_file(&self, file: &mut File) -> Result<(), RepoError> {
        match *self {
            ProtectedRefCounter::Immutable(_) => {
                panic!("{:?}: line {:?}: immutability breach", file!(), line!())
            }
            ProtectedRefCounter::Mutable(ref ref_counter) => {
                ref_counter.borrow().to_file(file)?;
            }
        }
        Ok(())
    }

    fn decr_ref_count_for_token(&self, token: &str) -> Result<RefCountData, RepoError> {
        match *self {
            ProtectedRefCounter::Immutable(_) => {
                panic!("{:?}: line {:?}: immutability breach", file!(), line!())
            }
            ProtectedRefCounter::Mutable(ref rc) => match rc.borrow_mut().0.get_mut(token) {
                Some(ref_count_data) => {
                    ref_count_data.ref_count -= 1;
                    Ok(*ref_count_data)
                }
                None => Err(RepoError::UnknownToken(token.to_string())),
            },
        }
    }

    fn incr_ref_count_for_token(&self, token: &str) -> Result<RefCountData, RepoError> {
        match *self {
            ProtectedRefCounter::Immutable(_) => {
                panic!("{:?}: line {:?}: immutability breach", file!(), line!())
            }
            ProtectedRefCounter::Mutable(ref rc) => match rc.borrow_mut().0.get_mut(token) {
                Some(ref_count_data) => {
                    ref_count_data.ref_count += 1;
                    Ok(*ref_count_data)
                }
                None => Err(RepoError::UnknownToken(token.to_string())),
            },
        }
    }

    fn insert(&self, token: &str, rcd: RefCountData) {
        match *self {
            ProtectedRefCounter::Immutable(_) => {
                panic!("{:?}: line {:?}: immutability breach", file!(), line!())
            }
            ProtectedRefCounter::Mutable(ref rc) => {
                rc.borrow_mut().0.insert(token.to_string(), rcd);
            }
        }
    }

    fn remove(&self, token: &str) {
        match *self {
            ProtectedRefCounter::Immutable(_) => {
                panic!("{:?}: line {:?}: immutability breach", file!(), line!())
            }
            ProtectedRefCounter::Mutable(ref rc) => {
                if let Some(rcd) = rc.borrow_mut().0.remove(token) {
                    if rcd.ref_count > 0 {
                        panic!("{:?}: line {:?}: attempt to remove non zero token", file!(), line!())
                    }
                }
            }
        }
    }
}

impl ProtectedRefCounter {
    // IMMUTABLE
    fn get_ref_count_data_for_token(&self, token: &str) -> Result<RefCountData, RepoError> {
        match *self {
            ProtectedRefCounter::Mutable(ref rc) => match rc.borrow().0.get(token) {
                Some(ref_count_data) => Ok(*ref_count_data),
                None => Err(RepoError::UnknownToken(token.to_string())),
            },
            ProtectedRefCounter::Immutable(ref hm) => match hm.0.get(token) {
                Some(ref_count_data) => Ok(*ref_count_data),
                None => Err(RepoError::UnknownToken(token.to_string())),
            },
        }
    }

    fn expired_tokens(&self) -> Vec<(String, RefCountData)> {
        match *self {
            ProtectedRefCounter::Mutable(ref rc) => rc.borrow().expired_tokens(),
            ProtectedRefCounter::Immutable(ref rc) => rc.expired_tokens(),
        }
    }
}

#[derive(Debug)]
pub struct ContentManager {
    content_mgmt_key: ContentMgmtKey,
    ref_counter: ProtectedRefCounter,
    hash_map_file: File,
}

impl Drop for ContentManager {
    fn drop(&mut self) {
        if self.ref_counter.is_mutable() {
            if let Err(err) = self.ref_counter.to_file(&mut self.hash_map_file) {
                panic!("{:?}: line {:?}: {:?}", file!(), line!(), err);
            };
        };
        if let Err(err) = self.hash_map_file.unlock() {
            panic!("{:?}: line {:?}: {:?}", file!(), line!(), err);
        };
    }
}

impl ContentManagerIfce for ContentManager {
    fn is_mutable(&self) -> bool {
        self.ref_counter.is_mutable()
    }

    //fn key<'a, K: ContentMgmtKeyIfce<'a, Self>>(&'a self) -> &'a K {
    //    &self.content_mgmt_key
    //}

    fn check_content_token<R: Read>(&self, reader: &mut R, token: &str) -> Result<bool, RepoError> {
        let digest = self
            .content_mgmt_key
            .hash_algortithm
            .reader_digest(reader)?;
        Ok(digest == token)
    }

    fn ref_count_for_token(&self, token: &str) -> Result<u64, RepoError> {
        let rcd = self.ref_counter.get_ref_count_data_for_token(token)?;
        Ok(rcd.ref_count)
    }

    fn write_contents_for_token<W: Write>(
        &self,
        content_token: &str,
        writer: &mut W,
    ) -> Result<u64, RepoError> {
        let content_file_path = self.content_mgmt_key.token_content_file_path(content_token);
        if !content_file_path.exists() {
            return Err(RepoError::UnknownToken(content_token.to_string()));
        }
        let content_file = File::open(content_file_path)?;
        let mut compressed_content_file = snap::Reader::new(content_file);
        let n = io::copy(&mut compressed_content_file, writer)?;
        Ok(n)
    }

    fn prune_contents(&self) -> Result<(u64, u64, u64), RepoError> {
        if !self.is_mutable() {
            panic!("{:?}: line {:?}: immutability breach", file!(), line!());
        }
        let mut content_sum = 0;
        let mut stored_sum = 0;
        let expired_tokens = self.ref_counter.expired_tokens();
        for (token, rcd) in expired_tokens.iter() {
            let path = self.content_mgmt_key.token_content_file_path(token);
            remove_file(&path)?;
            content_sum += rcd.content_size;
            stored_sum += rcd.stored_size;
            self.ref_counter.remove(token);
        }
        Ok((expired_tokens.len() as u64, content_sum, stored_sum))
    }

    fn release_contents(&self, content_token: &str) -> Result<RefCountData, RepoError> {
        self.ref_counter.decr_ref_count_for_token(&content_token)
    }

    fn store_contents(&self, file: &mut File) -> Result<(String, u64, u64), RepoError> {
        let digest = self.content_mgmt_key.hash_algortithm.reader_digest(file)?;
        match self.ref_counter.incr_ref_count_for_token(&digest) {
            Ok(rcd) => Ok((digest, rcd.stored_size, 0)),
            Err(_) => {
                let content_size = match file.metadata() {
                    Ok(metadata) => metadata.len(),
                    Err(err) => panic!("{:?}: line {:?}: {:?}", file!(), line!(), err),
                };
                let content_file_path = self.content_mgmt_key.token_content_file_path(&digest);
                let content_dir_path = content_file_path
                    .parent()
                    .expect("Failed to extract content directory path");
                if !content_dir_path.exists() {
                    create_dir_all(content_dir_path)?;
                }
                file.seek(io::SeekFrom::Start(0))?;
                let content_file = File::create(&content_file_path)?;
                let mut compressed_content_file = snap::Writer::new(content_file);
                io::copy(file, &mut compressed_content_file)?;
                let metadata = content_file_path.metadata()?;
                let stored_size = metadata.len();
                let rcd = RefCountData {
                    content_size: content_size,
                    stored_size: stored_size,
                    ref_count: 1,
                };
                self.ref_counter.insert(&digest, rcd);
                Ok((digest, stored_size, stored_size))
            }
        }
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
            "1CF251472D59F8FADEB3AB258E90999D8491BE19".to_string(),
        );
        let mut file = File::open("./LICENSE-APACHE").unwrap();
        assert_eq!(
            hash_algorithm.reader_digest(&mut file).unwrap(),
            "7DF059597099BB7DCF25D2A9AEDFAF4465F72D8D".to_string(),
        );
    }

    #[test]
    fn repo_use() {
        let tmp_dir = TempDir::new("TEST").unwrap();
        let repo_dir = tmp_dir.path().join("repo");
        let repo_spec = RepoSpec::new(&repo_dir, HashAlgorithm::Sha1);
        let cm_key: ContentMgmtKey = (&repo_spec).into();
        assert!(cm_key.create_repo_dir().is_ok());
        let cmgr = cm_key.open_content_manager(true).unwrap();
        let mut file = File::open("./LICENSE-APACHE").unwrap();
        let result = cmgr.store_contents(&mut file).unwrap();
        assert_eq!(
            result.0,
            "7DF059597099BB7DCF25D2A9AEDFAF4465F72D8D".to_string(),
        );
        assert_eq!(cmgr.ref_count_for_token(&result.0).unwrap(), 1);
        let mut file = File::open("./LICENSE-APACHE").unwrap();
        let result = cmgr.store_contents(&mut file).unwrap();
        assert_eq!(cmgr.ref_count_for_token(&result.0).unwrap(), 2);
        assert!(cmgr.release_contents(&result.0).is_ok());
        assert_eq!(cmgr.ref_count_for_token(&result.0).unwrap(), 1);
        let target_path = tmp_dir.path().join("target");
        let mut target_file = File::create(&target_path).unwrap();
        assert!(cmgr
            .write_contents_for_token(&result.0, &mut target_file)
            .is_ok());
        let f1 = File::open(&target_path).unwrap();
        let f2 = File::open("./LICENSE-APACHE").unwrap();
        for (b1, b2) in f1.bytes().zip(f2.bytes()) {
            assert_eq!(b1.unwrap(), b2.unwrap());
        }
        assert!(cmgr.release_contents(&result.0).is_ok());
        assert_eq!(cmgr.ref_count_for_token(&result.0).unwrap(), 0);
        assert!(cmgr.prune_contents().is_ok());
        assert!(cmgr.ref_count_for_token(&result.0).is_err());
    }
}
