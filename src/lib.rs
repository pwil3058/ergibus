#[macro_use]
extern crate serde_derive;

use std::{
    cell::RefCell,
    collections::HashMap,
    fs::{create_dir_all, remove_file, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    ops::AddAssign,
    path::{Path, PathBuf},
    str::FromStr,
};

use crypto_hash;
use fs2::FileExt;
use hex::ToHex;
//use serde::{Deserialize, Serialize};
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

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Mutability {
    Immutable,
    Mutable,
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

    pub fn open_content_manager(
        &self,
        mutability: Mutability,
    ) -> Result<ContentManager, RepoError> {
        let mut hash_map_file = self.locked_ref_count_file(mutability)?;
        let ref_counter = ProtectedRefCounter::from_file(&mut hash_map_file, mutability)?;
        let storage = Storage {
            base_dir_path: self.base_dir_path.clone(),
        };
        Ok(ContentManager {
            content_mgmt_key: self.clone(),
            ref_counter,
            storage,
            hash_map_file,
        })
    }

    fn locked_ref_count_file(&self, mutability: Mutability) -> Result<File, RepoError> {
        let mutable = mutability == Mutability::Mutable;
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
    stored_size: u64,
}

impl RefCountData {
    fn decr_ref_count(&mut self) {
        if self.ref_count > 0 {
            self.ref_count -= 1;
        } else {
            panic!(
                "{:?}: line {:?}: decrement zero ref count",
                file!(),
                line!()
            )
        }
    }

    fn incr_ref_count(&mut self) {
        self.ref_count += 1;
    }
}

#[derive(PartialEq, Clone, Copy, Default, Debug)]
pub struct UnreferencedContentData {
    num_items: u64,
    sum_content: u128,
    sum_storage: u128,
}

impl AddAssign<&RefCountData> for UnreferencedContentData {
    fn add_assign(&mut self, ref_count_data: &RefCountData) {
        if ref_count_data.ref_count == 0 {
            self.num_items += 1;
            self.sum_content += ref_count_data.content_size as u128;
            self.sum_storage += ref_count_data.stored_size as u128;
        }
    }
}

#[derive(PartialEq, Clone, Copy, Default, Debug)]
pub struct ReferencedContentData {
    num_items: u64,
    num_references: u128,
    sum_content: u128,
    sum_notional_content: u128,
    sum_storage: u128,
}

impl AddAssign<&RefCountData> for ReferencedContentData {
    fn add_assign(&mut self, ref_count_data: &RefCountData) {
        if ref_count_data.ref_count > 0 {
            self.num_items += 1;
            self.num_references += ref_count_data.ref_count as u128;
            self.sum_content += ref_count_data.content_size as u128;
            self.sum_notional_content +=
                ref_count_data.content_size as u128 * ref_count_data.ref_count as u128;
            self.sum_storage += ref_count_data.stored_size as u128;
        }
    }
}

#[derive(PartialEq, Clone, Copy, Default, Debug)]
pub struct ContentData {
    referenced_content_data: ReferencedContentData,
    unreferenced_content_data: UnreferencedContentData,
}

impl AddAssign<&RefCountData> for ContentData {
    fn add_assign(&mut self, ref_count_data: &RefCountData) {
        if ref_count_data.ref_count > 0 {
            self.referenced_content_data += ref_count_data;
        } else {
            self.unreferenced_content_data += ref_count_data;
        }
    }
}

#[derive(Debug)]
pub enum TokenProblem {
    ContentMissing(String),
    ContentInconsistent(String),
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

    fn unreferenced_tokens(&self) -> Vec<String> {
        self.0
            .iter()
            .filter(|(_, rcd)| rcd.ref_count == 0)
            .map(|(t, _)| t.clone())
            .collect()
    }

    fn insert(&mut self, token: &str, rcd: RefCountData) {
        self.0.insert(token.to_string(), rcd);
    }

    fn remove(&mut self, token: &str) -> Result<RefCountData, RepoError> {
        if let Some(rcd) = self.0.remove(token) {
            if rcd.ref_count > 0 {
                panic!(
                    "{:?}: line {:?}: attempt to remove non zero token",
                    file!(),
                    line!()
                )
            };
            Ok(rcd)
        } else {
            Err(RepoError::UnknownToken(token.to_string()))
        }
    }

    fn decr_ref_count(&mut self, token: &str) -> Result<RefCountData, RepoError> {
        match self.0.get_mut(token) {
            Some(ref_count_data) => {
                ref_count_data.decr_ref_count();
                Ok(*ref_count_data)
            }
            None => Err(RepoError::UnknownToken(token.to_string())),
        }
    }

    fn incr_ref_count(&mut self, token: &str) -> Result<RefCountData, RepoError> {
        match self.0.get_mut(token) {
            Some(ref_count_data) => {
                ref_count_data.incr_ref_count();
                Ok(*ref_count_data)
            }
            None => Err(RepoError::UnknownToken(token.to_string())),
        }
    }

    fn ref_count_data_for_token(&self, token: &str) -> Result<RefCountData, RepoError> {
        match self.0.get(token) {
            Some(ref_count_data) => Ok(*ref_count_data),
            None => Err(RepoError::UnknownToken(token.to_string())),
        }
    }

    fn unreferenced_content_data(&self) -> UnreferencedContentData {
        let mut data = UnreferencedContentData::default();
        for ref_count_data in self.0.values() {
            data += ref_count_data
        }
        data
    }

    fn referenced_content_data(&self) -> ReferencedContentData {
        let mut data = ReferencedContentData::default();
        for ref_count_data in self.0.values() {
            data += ref_count_data
        }
        data
    }

    fn content_data(&self) -> ContentData {
        let mut data = ContentData::default();
        for ref_count_data in self.0.values() {
            data += ref_count_data
        }
        data
    }

    fn token_problems(&self, storage: &Storage) -> Vec<TokenProblem> {
        let mut problems = vec![];
        for (token, ref_count_data) in self.0.iter() {
            if let Ok(len) = storage.stored_size(token) {
                if ref_count_data.stored_size != len {
                    problems.push(TokenProblem::ContentInconsistent(token.clone()));
                }
            } else {
                problems.push(TokenProblem::ContentMissing(token.clone()));
            }
        }
        problems
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

    fn from_file(
        file: &mut File,
        mutability: Mutability,
    ) -> Result<ProtectedRefCounter, RepoError> {
        let ref_counter = RefCounter::from_file(file)?;
        if mutability == Mutability::Mutable {
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
            ProtectedRefCounter::Mutable(ref rc) => rc.borrow_mut().decr_ref_count(token),
        }
    }

    fn incr_ref_count_for_token(&self, token: &str) -> Result<RefCountData, RepoError> {
        match *self {
            ProtectedRefCounter::Immutable(_) => {
                panic!("{:?}: line {:?}: immutability breach", file!(), line!())
            }
            ProtectedRefCounter::Mutable(ref rc) => rc.borrow_mut().incr_ref_count(token),
        }
    }

    fn insert(&self, token: &str, rcd: RefCountData) {
        match *self {
            ProtectedRefCounter::Immutable(_) => {
                panic!("{:?}: line {:?}: immutability breach", file!(), line!())
            }
            ProtectedRefCounter::Mutable(ref rc) => {
                rc.borrow_mut().insert(token, rcd);
            }
        }
    }

    fn remove(&self, token: &str) -> Result<RefCountData, RepoError> {
        match *self {
            ProtectedRefCounter::Immutable(_) => {
                panic!("{:?}: line {:?}: immutability breach", file!(), line!())
            }
            ProtectedRefCounter::Mutable(ref rc) => rc.borrow_mut().remove(token),
        }
    }
}

impl ProtectedRefCounter {
    // IMMUTABLE
    fn ref_count_data_for_token(&self, token: &str) -> Result<RefCountData, RepoError> {
        match *self {
            ProtectedRefCounter::Mutable(ref rc) => rc.borrow().ref_count_data_for_token(token),
            ProtectedRefCounter::Immutable(ref rc) => rc.ref_count_data_for_token(token),
        }
    }

    fn unreferenced_tokens(&self) -> Vec<String> {
        match *self {
            ProtectedRefCounter::Mutable(ref rc) => rc.borrow().unreferenced_tokens(),
            ProtectedRefCounter::Immutable(ref rc) => rc.unreferenced_tokens(),
        }
    }

    fn unreferenced_content_data(&self) -> UnreferencedContentData {
        match *self {
            ProtectedRefCounter::Mutable(ref rc) => rc.borrow().unreferenced_content_data(),
            ProtectedRefCounter::Immutable(ref rc) => rc.unreferenced_content_data(),
        }
    }

    fn referenced_content_data(&self) -> ReferencedContentData {
        match *self {
            ProtectedRefCounter::Mutable(ref rc) => rc.borrow().referenced_content_data(),
            ProtectedRefCounter::Immutable(ref rc) => rc.referenced_content_data(),
        }
    }

    fn content_data(&self) -> ContentData {
        match *self {
            ProtectedRefCounter::Mutable(ref rc) => rc.borrow().content_data(),
            ProtectedRefCounter::Immutable(ref rc) => rc.content_data(),
        }
    }

    fn token_problems(&self, storage: &Storage) -> Vec<TokenProblem> {
        match *self {
            ProtectedRefCounter::Mutable(ref rc) => rc.borrow().token_problems(storage),
            ProtectedRefCounter::Immutable(ref rc) => rc.token_problems(storage),
        }
    }
}

#[derive(Debug)]
pub struct Storage {
    base_dir_path: PathBuf,
}

pub enum ContentProblem {
    Orphaned(String),
    Inconsistent(String),
}

impl Storage {
    fn token_content_file_path(&self, token: &str) -> PathBuf {
        let mut path_buf = self.base_dir_path.clone();
        path_buf.push(PathBuf::from(&token[0..3]));
        path_buf.push(PathBuf::from(&token[3..]));

        path_buf
    }

    fn store(&self, token: &str, file: &mut File) -> Result<u64, RepoError> {
        let content_file_path = self.token_content_file_path(token);
        let content_dir_path = content_file_path
            .parent()
            .expect("Failed to extract content directory path");
        if !content_dir_path.exists() {
            create_dir_all(content_dir_path)?;
        }
        let content_file = File::create(&content_file_path)?;
        let mut compressed_content_file = snap::Writer::new(content_file);
        io::copy(file, &mut compressed_content_file)?;
        compressed_content_file.flush()?;
        let metadata = content_file_path.metadata()?;
        Ok(metadata.len())
    }

    fn remove(&self, token: &str) -> Result<(), RepoError> {
        let path = self.token_content_file_path(token);
        remove_file(&path)?;
        Ok(())
    }

    fn write<W: Write>(&self, content_token: &str, writer: &mut W) -> Result<u64, RepoError> {
        let content_file_path = self.token_content_file_path(content_token);
        if !content_file_path.exists() {
            return Err(RepoError::UnknownToken(content_token.to_string()));
        }
        let content_file = File::open(content_file_path)?;
        let mut compressed_content_file = snap::Reader::new(content_file);
        let n = io::copy(&mut compressed_content_file, writer)?;
        Ok(n)
    }

    fn stored_size(&self, token: &str) -> Result<u64, RepoError> {
        let content_file_path = self.token_content_file_path(token);
        let metadata = content_file_path.metadata()?;
        Ok(metadata.len())
    }

    fn content_problems(
        &self,
        ref_counter: &ProtectedRefCounter,
    ) -> Result<Vec<ContentProblem>, RepoError> {
        let mut problems = vec![];
        for r_tl_entry in self.base_dir_path.read_dir()? {
            let tl_entry = r_tl_entry?;
            if tl_entry.file_type()?.is_dir() {
                let dir_name = tl_entry.file_name().into_string()?;
                for r_sl_entry in self.base_dir_path.join(&dir_name).read_dir()? {
                    let sl_entry = r_sl_entry?;
                    if sl_entry.file_type()?.is_file() {
                        let mut token = dir_name.clone();
                        token.push_str(&sl_entry.file_name().into_string()?);
                        if let Ok(ref_count_data) = ref_counter.ref_count_data_for_token(&token) {
                            if ref_count_data.stored_size != sl_entry.metadata()?.len() {
                                problems.push(ContentProblem::Inconsistent(token));
                            }
                        } else {
                            problems.push(ContentProblem::Orphaned(token));
                        }
                    }
                }
            }
        }
        Ok(problems)
    }
}

#[derive(Debug)]
pub struct ContentManager {
    content_mgmt_key: ContentMgmtKey,
    ref_counter: ProtectedRefCounter,
    storage: Storage,
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

pub struct Problems {
    pub token_problems: Vec<TokenProblem>,
    pub content_problems: Vec<ContentProblem>,
}

impl ContentManager {
    pub fn is_mutable(&self) -> bool {
        self.ref_counter.is_mutable()
    }

    pub fn key<'a>(&'a self) -> &'a ContentMgmtKey {
        &self.content_mgmt_key
    }

    pub fn check_content_token<R: Read>(
        &self,
        reader: &mut R,
        token: &str,
    ) -> Result<bool, RepoError> {
        let digest = self
            .content_mgmt_key
            .hash_algortithm
            .reader_digest(reader)?;
        Ok(digest == token)
    }

    pub fn content_data(&self) -> ContentData {
        self.ref_counter.content_data()
    }

    pub fn referenced_content_data(&self) -> ReferencedContentData {
        self.ref_counter.referenced_content_data()
    }

    pub fn unreferenced_content_data(&self) -> UnreferencedContentData {
        self.ref_counter.unreferenced_content_data()
    }

    pub fn ref_count_for_token(&self, token: &str) -> Result<u64, RepoError> {
        let rcd = self.ref_counter.ref_count_data_for_token(token)?;
        Ok(rcd.ref_count)
    }

    pub fn write_contents_for_token<W: Write>(
        &self,
        content_token: &str,
        writer: &mut W,
    ) -> Result<u64, RepoError> {
        let n = self.storage.write(content_token, writer)?;
        Ok(n)
    }

    pub fn prune_contents(&self) -> Result<UnreferencedContentData, RepoError> {
        if !self.is_mutable() {
            panic!("{:?}: line {:?}: immutability breach", file!(), line!());
        }
        let mut unreferenced_content_data = UnreferencedContentData::default();
        let unreferenced_tokens = self.ref_counter.unreferenced_tokens();
        for token in unreferenced_tokens.iter() {
            self.storage.remove(token)?;
            unreferenced_content_data += &self.ref_counter.remove(token)?;
        }
        Ok(unreferenced_content_data)
    }

    pub fn release_contents(&self, content_token: &str) -> Result<RefCountData, RepoError> {
        self.ref_counter.decr_ref_count_for_token(&content_token)
    }

    pub fn store_contents(&self, file: &mut File) -> Result<(String, u64, u64), RepoError> {
        let digest = self.content_mgmt_key.hash_algortithm.reader_digest(file)?;
        match self.ref_counter.incr_ref_count_for_token(&digest) {
            Ok(rcd) => Ok((digest, rcd.stored_size, 0)),
            Err(_) => {
                // NB: reader_digest will have moved the pointer
                file.seek(io::SeekFrom::Start(0))?;
                let content_size = match file.metadata() {
                    Ok(metadata) => metadata.len(),
                    Err(err) => panic!("{:?}: line {:?}: {:?}", file!(), line!(), err),
                };
                let stored_size = self.storage.store(&digest, file)?;
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

    pub fn problems(&self) -> Result<Problems, RepoError> {
        let token_problems = self.ref_counter.token_problems(&self.storage);
        let content_problems = self.storage.content_problems(&self.ref_counter)?;
        Ok(Problems {
            token_problems,
            content_problems,
        })
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
        let cmgr = cm_key.open_content_manager(Mutability::Mutable).unwrap();
        assert_eq!(
            cmgr.unreferenced_content_data(),
            UnreferencedContentData::default()
        );
        assert_eq!(
            cmgr.referenced_content_data(),
            ReferencedContentData::default()
        );
        let mut file = File::open("./LICENSE-APACHE").unwrap();
        let result = cmgr.store_contents(&mut file).unwrap();
        assert_eq!(
            result.0,
            "7DF059597099BB7DCF25D2A9AEDFAF4465F72D8D".to_string(),
        );
        assert_eq!(cmgr.ref_count_for_token(&result.0).unwrap(), 1);
        assert_eq!(
            cmgr.unreferenced_content_data(),
            UnreferencedContentData::default()
        );
        let expected = ReferencedContentData {
            num_items: 1,
            num_references: 1,
            sum_content: 11357,
            sum_notional_content: 11357,
            sum_storage: 5816,
        };
        assert_eq!(cmgr.referenced_content_data(), expected);
        let mut file = File::open("./LICENSE-APACHE").unwrap();
        let result = cmgr.store_contents(&mut file).unwrap();
        assert_eq!(cmgr.ref_count_for_token(&result.0).unwrap(), 2);
        assert_eq!(
            cmgr.unreferenced_content_data(),
            UnreferencedContentData::default()
        );
        let expected = ReferencedContentData {
            num_items: 1,
            num_references: 2,
            sum_content: 11357,
            sum_notional_content: 22714,
            sum_storage: 5816,
        };
        assert_eq!(cmgr.referenced_content_data(), expected);
        assert!(cmgr.release_contents(&result.0).is_ok());
        assert_eq!(cmgr.ref_count_for_token(&result.0).unwrap(), 1);
        assert_eq!(
            cmgr.unreferenced_content_data(),
            UnreferencedContentData::default()
        );
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
        let expected = UnreferencedContentData {
            num_items: 1,
            sum_content: 11357,
            sum_storage: 5816,
        };
        assert_eq!(cmgr.unreferenced_content_data(), expected);
        assert_eq!(
            cmgr.referenced_content_data(),
            ReferencedContentData::default()
        );
        assert_eq!(cmgr.prune_contents().unwrap(), expected);
        assert!(cmgr.ref_count_for_token(&result.0).is_err());
    }
}
