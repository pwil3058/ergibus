// Copyright 2017 Peter Williams <pwil3058@gmail.com>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions, create_dir_all};
use std::io::prelude::*;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use fs2::FileExt;

use hex::ToHex;

use crypto_hash;
use serde_json;
use serde_yaml;
use snap;

use attributes::{Attributes, AttributesIfce};
use config;
use eerror::{EError, EResult};

pub fn content_repo_exists(repo_name: &str) -> bool {
    get_repo_spec_file_path(repo_name).exists()
}

pub fn get_content_mgmt_key(repo_name: &str) -> EResult<ContentMgmtKey> {
    if !content_repo_exists(repo_name) {
        Err(EError::UnknownRepo(repo_name.to_string()))
    } else {
        let spec = read_repo_spec(repo_name)?;
        Ok(ContentMgmtKey::from(&spec))
    }
}

pub fn create_new_repo(name: &str, location: &str, hash_algortithm_str: &str) -> EResult<()> {
    if content_repo_exists(name) {
        return Err(EError::RepoExists(name.to_string()))
    }

    let hash_algorithm = HashAlgorithm::from_str(hash_algortithm_str)?;

    let mut repo_dir_path = PathBuf::from(location);
    repo_dir_path.push("ergibus");
    repo_dir_path.push("repos");
    repo_dir_path.push(name);
    fs::create_dir_all(&repo_dir_path).map_err(|err| EError::RepoCreateError(err, repo_dir_path.clone()))?;

    let spec = RepoSpec {
        base_dir_path: repo_dir_path,
        hash_algorithm: hash_algorithm
    };

    let key = ContentMgmtKey::from(&spec);
    let mut file = File::create(&key.ref_counter_path).map_err(|err| EError::RefCounterWriteIOError(err))?;
    write_ref_count_hash_map(&mut file, &RefCountHashMap::new())?;

    write_repo_spec(name, &spec)?;
    Ok(())
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub enum HashAlgorithm {
    Sha1,
    Sha256,
    Sha512,
}

impl FromStr for HashAlgorithm {
    type Err = EError;
    fn from_str(src: &str) -> Result<HashAlgorithm, EError> {
        match src {
            "Sha1" | "SHA1" | "sha1" => Ok(HashAlgorithm::Sha1),
            "Sha256" | "SHA256" | "sha256" => Ok(HashAlgorithm::Sha256),
            "Sha512" | "SHA512" | "sha512" => Ok(HashAlgorithm::Sha512),
            _ => Err(EError::UnknownKeyAlgorithm(src.to_string())),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct RepoSpec {
    base_dir_path: PathBuf,
    hash_algorithm: HashAlgorithm
}

fn get_repo_spec_file_path(repo_name: &str) -> PathBuf {
    config::get_repo_config_dir_path().join(repo_name)
}

fn read_repo_spec(repo_name: &str) -> EResult<RepoSpec> {
    let spec_file_path = get_repo_spec_file_path(repo_name);
    let spec_file = File::open(&spec_file_path).map_err(|err| EError::RepoReadError(err, spec_file_path.clone()))?;
    let spec: RepoSpec = serde_yaml::from_reader(&spec_file).map_err(|err| EError::RepoYamlReadError(err, repo_name.to_string()))?;
    Ok(spec)
}

fn write_repo_spec(repo_name: &str, repo_spec: &RepoSpec) -> EResult<()> {
    let spec_file_path = get_repo_spec_file_path(repo_name);
    if spec_file_path.exists() {
        return Err(EError::RepoExists(repo_name.to_string()))
    }
    match spec_file_path.parent() {
        Some(config_dir_path) => if !config_dir_path.exists() {
            fs::create_dir_all(&config_dir_path).map_err(|err| EError::RepoWriteError(err, config_dir_path.to_path_buf()))?;
        },
        None => (),
    }
    let spec_file = File::create(&spec_file_path).map_err(|err| EError::RepoWriteError(err, spec_file_path.clone()))?;
    serde_yaml::to_writer(&spec_file, repo_spec).map_err(|err| EError::RepoYamlWriteError(err, repo_name.to_string()))?;
    Ok(())
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
    pub fn new_dummy() -> ContentMgmtKey {
        ContentMgmtKey {
            base_dir_path: PathBuf::from("whatever"),
            ref_counter_path: PathBuf::from("whatever"),
            hash_algortithm: HashAlgorithm::Sha1,
        }
    }

    fn locked_ref_count_file(&self, mutable: bool) -> EResult<File> {
        let file = OpenOptions::new()
                    .read(true)
                    .write(mutable)
                    .open(&self.ref_counter_path).map_err(|err| EError::RefCounterReadIOError(err))?;
        if mutable {
            file.lock_exclusive().map_err(|err| EError::RefCounterReadIOError(err))?;
        } else {
            file.lock_shared().map_err(|err| EError::RefCounterReadIOError(err))?;
        }
        Ok(file)
    }

    pub fn open_content_manager(&self, mutable: bool) -> EResult<ContentManager> {
        let mut hash_map_file = self.locked_ref_count_file(mutable)?;
        let ref_counter = ProtectedRefCounter::from_file(&mut hash_map_file, mutable)?;
        Ok(ContentManager{
            content_mgmt_key: self.clone(),
            ref_counter: ref_counter,
            hash_map_file: hash_map_file
        })
    }

    fn token_content_file_path(&self, token: &str) -> PathBuf {
        let mut path_buf = self.base_dir_path.clone();
        path_buf.push(PathBuf::from(&token[0..3]));
        path_buf.push(PathBuf::from(&token[3..]));

        path_buf
    }
}

fn file_digest(hash_algorithm: HashAlgorithm, file: &mut File) -> Result<String, io::Error> {
    let mut buffer = [0; 512000];
    let mut hasher = match hash_algorithm {
        HashAlgorithm::Sha1 => crypto_hash::Hasher::new(crypto_hash::Algorithm::SHA1),
        HashAlgorithm::Sha256 => crypto_hash::Hasher::new(crypto_hash::Algorithm::SHA256),
        HashAlgorithm::Sha512 => crypto_hash::Hasher::new(crypto_hash::Algorithm::SHA512),
    };
    loop {
        let n_bytes = file.read(&mut buffer)?;
        if n_bytes == 0 {
            break;
        };
        hasher.write_all(&buffer[..n_bytes])?;
    }
    Ok(hasher.finish().to_hex())
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub struct RefCountData {
    ref_count: u64,
    content_size: u64,
    stored_size: u64
}

type RefCountHashMap = HashMap<String, RefCountData>;

fn read_ref_count_hash_map(file: &mut File) -> EResult<RefCountHashMap> {
    let mut rchp_str = String::new();
    let mut snappy_rdr = snap::Reader::new(file);
    snappy_rdr.read_to_string(&mut rchp_str).map_err(|err| EError::RefCounterReadIOError(err))?;
    let rchp = serde_json::from_str::<RefCountHashMap>(&rchp_str).map_err(|err| EError::RefCounterReadJsonError(err))?;
    Ok(rchp)
}

fn write_ref_count_hash_map(file: &mut File, hash_map: &RefCountHashMap) -> EResult<()> {
    let json_text = serde_json::to_string(hash_map).map_err(|err| EError::RefCounterSerializeError(err))?;
    file.seek(io::SeekFrom::Start(0)).map_err(|err| EError::RefCounterWriteIOError(err))?;
    file.set_len(0).map_err(|err| EError::RefCounterWriteIOError(err))?;
    let mut snappy_wtr = snap::Writer::new(file);
    snappy_wtr.write_all(json_text.as_bytes()).map_err(|err| EError::RefCounterWriteIOError(err))?;
    Ok(())
}

#[derive(Debug)]
enum ProtectedRefCounter {
    Immutable(RefCountHashMap),
    Mutable(RefCell<RefCountHashMap>)
}

impl ProtectedRefCounter { // GENERAL
    fn is_mutable(&self) -> bool {
        match *self {
            ProtectedRefCounter::Immutable(_) => false,
            ProtectedRefCounter::Mutable(_) => true
        }
    }

    fn from_file(file: &mut File, mutable: bool) -> EResult<ProtectedRefCounter> {
        let rchp = read_ref_count_hash_map(file)?;
        if mutable {
            Ok(ProtectedRefCounter::Mutable(RefCell::new(rchp)))
        } else {
            Ok(ProtectedRefCounter::Immutable(rchp))
        }
    }
}

impl ProtectedRefCounter { // MUTABLE
    fn write_to_file(&self, file: &mut File) -> EResult<()> {
        match *self {
            ProtectedRefCounter::Immutable(_) => panic!("{:?}: line {:?}: immutability breach", file!(), line!()),
            ProtectedRefCounter::Mutable(ref rc) => {
                write_ref_count_hash_map(file, &rc.borrow())?;
            }
        }
        Ok(())
    }


    fn incr_ref_count_for_token(&self, token: &str) -> EResult<RefCountData> {
        match *self {
            ProtectedRefCounter::Immutable(_) => panic!("{:?}: line {:?}: immutability breach", file!(), line!()),
            ProtectedRefCounter::Mutable(ref rc) => {
                match rc.borrow_mut().get_mut(token) {
                    Some(ref_count_data) => {
                        ref_count_data.ref_count += 1;
                        Ok(*ref_count_data)
                    },
                    None => Err(EError::UnknownContentKey(token.to_string()))
                }

            }
        }
    }

    fn decr_ref_count_for_token(&self, token: &str) -> EResult<RefCountData> {
        match *self {
            ProtectedRefCounter::Immutable(_) => panic!("{:?}: line {:?}: immutability breach", file!(), line!()),
            ProtectedRefCounter::Mutable(ref rc) => {
                match rc.borrow_mut().get_mut(token) {
                    Some(ref_count_data) => {
                        ref_count_data.ref_count -= 1;
                        Ok(*ref_count_data)
                    },
                    None => Err(EError::UnknownContentKey(token.to_string()))
                }
            }
        }
    }

    fn insert(&self, token: &str, rcd: RefCountData) {
        match *self {
            ProtectedRefCounter::Immutable(_) => panic!("{:?}: line {:?}: immutability breach", file!(), line!()),
            ProtectedRefCounter::Mutable(ref rc) => {
                rc.borrow_mut().insert(token.to_string(), rcd);
            }
        }
    }
}

impl ProtectedRefCounter { // IMMUTABLE
    fn get_ref_count_data_for_token(&self, token: &str) -> EResult<RefCountData> {
        match *self {
            ProtectedRefCounter::Mutable(ref rc) => {
                match rc.borrow().get(token) {
                    Some(ref_count_data) => {
                        Ok(*ref_count_data)
                    },
                    None => Err(EError::UnknownContentKey(token.to_string()))
                }
            },
            ProtectedRefCounter::Immutable(ref hm) => {
                match hm.get(token) {
                    Some(ref_count_data) => {
                        Ok(*ref_count_data)
                    },
                    None => Err(EError::UnknownContentKey(token.to_string()))
                }
            },
        }
    }
}

#[derive(Debug)]
pub struct ContentManager {
    content_mgmt_key: ContentMgmtKey,
    ref_counter: ProtectedRefCounter,
    hash_map_file: File
}

impl Drop for ContentManager {
    fn drop(&mut self) {
        if self.ref_counter.is_mutable() {
            if let Err(err) = self.ref_counter.write_to_file(&mut self.hash_map_file) {
                panic!("{:?}: line {:?}: {:?}", file!(), line!(), err);
            };
        };
        if let Err(err) = self.hash_map_file.unlock() {
            panic!("{:?}: line {:?}: {:?}", file!(), line!(), err);
        };
    }
}

impl ContentManager {
    pub fn store_file_contents(&self, abs_file_path: &Path) -> EResult<(String, u64, u64)> {
        let mut file = File::open(abs_file_path).map_err(|err| EError::ContentStoreIOError(err))?;
        let digest = file_digest(self.content_mgmt_key.hash_algortithm, &mut file).map_err(|err| EError::ContentStoreIOError(err))?;
        match self.ref_counter.incr_ref_count_for_token(&digest) {
            Ok(rcd) => Ok((digest, rcd.stored_size, 0)),
            Err(_) => {
                let content_size = match file.metadata() {
                    Ok(metadata) => metadata.len(),
                    Err(err) => panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
                };
                let content_file_path = self.content_mgmt_key.token_content_file_path(&digest);
                let content_dir_path = content_file_path.parent().expect("Failed to extract content directory path");
                if !content_dir_path.exists() {
                    create_dir_all(content_dir_path).map_err(|err| EError::ContentStoreIOError(err))?;
                }
                file.seek(io::SeekFrom::Start(0)).map_err(|err| EError::ContentStoreIOError(err))?;
                let content_file = File::create(&content_file_path).map_err(|err| EError::ContentStoreIOError(err))?;
                let mut compressed_content_file = snap::Writer::new(content_file);
                io::copy(&mut file, &mut compressed_content_file).map_err(|err| EError::ContentStoreIOError(err))?;
                let metadata = content_file_path.metadata().map_err(|err| EError::ContentStoreIOError(err))?;
                let stored_size = metadata.len();
                let rcd = RefCountData{
                    content_size: content_size,
                    stored_size: stored_size,
                    ref_count: 1
                };
                self.ref_counter.insert(&digest, rcd);
                Ok((digest, stored_size, stored_size))
            }
        }
    }

    pub fn copy_contents_for_token<W>(&self, content_token: &str, target_path: &Path, attributes: &Attributes, op_errf: &mut Option<&mut W>) -> EResult<u64>
        where W: std::io::Write
    {
        let content_file_path = self.content_mgmt_key.token_content_file_path(content_token);
        if !content_file_path.exists() {
            return Err(EError::UnknownContentKey(content_token.to_string()));
        }
        let mut target_file = File::create(target_path).map_err(|err| EError::ContentCopyIOError(err))?;
        let content_file = File::open(content_file_path).map_err(|err| EError::ContentCopyIOError(err))?;
        let mut compressed_content_file = snap::Reader::new(content_file);
        let n = io::copy(&mut compressed_content_file, &mut target_file).map_err(|err| EError::ContentStoreIOError(err))?;
        attributes.set_file_attributes(target_path, op_errf).map_err(|err| EError::ContentCopyIOError(err))?;
        Ok(n)
    }

    pub fn read_contents_for_token(&self, content_token: &str) -> EResult<Vec<u8>> {
        let content_file_path = self.content_mgmt_key.token_content_file_path(content_token);
        if !content_file_path.exists() {
            return Err(EError::UnknownContentKey(content_token.to_string()));
        }
        let mut contents = Vec::<u8>::new();
        let content_file = File::open(content_file_path).map_err(|err| EError::ContentReadIOError(err))?;
        let mut compressed_content_file = snap::Reader::new(content_file);
        compressed_content_file.read_to_end(&mut contents).map_err(|err| EError::ContentReadIOError(err))?;
        Ok(contents)
    }

    pub fn release_contents(&self, content_token: &str) -> EResult<RefCountData> {
        self.ref_counter.decr_ref_count_for_token(&content_token)
    }

    pub fn get_ref_count_for_token(&self, token: &str) -> EResult<u64> {
        let rcd = self.ref_counter.get_ref_count_data_for_token(token)?;
        Ok(rcd.ref_count)
    }

    pub fn check_content_token(&self, file_path: &Path, token: &str) -> EResult<bool> {
        let mut file = File::open(file_path).map_err(|err| EError::ContentStoreIOError(err))?;
        let digest = file_digest(self.content_mgmt_key.hash_algortithm, &mut file).map_err(|err| EError::ContentStoreIOError(err))?;
        Ok(digest == token)
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use tempdir::TempDir;
    use super::*;

    #[test]
    fn repo_works() {
        let file = OpenOptions::new().write(true).open("./test_lock_file").unwrap_or_else (
            |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
        );
        if let Err(err) = file.lock_exclusive() {
            panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
        };
        let temp_dir = TempDir::new("REPO_TEST").unwrap_or_else(
            |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
        );
        env::set_var("ERGIBUS_CONFIG_DIR", temp_dir.path().join("config"));
        let data_dir = temp_dir.path().join("data");
        let data_dir_str = match data_dir.to_str() {
            Some(data_dir_str) => data_dir_str,
            None => panic!("{:?}: line {:?}", file!(), line!())
        };
        if let Err(err) = create_new_repo("test_repo", data_dir_str, "Sha1") {
            panic!("new repo: {:?}", err);
        }
        assert!(temp_dir.path().join("config").join("repos").join("test_repo").exists());
        assert!(temp_dir.path().join("data").join("ergibus").join("repos").join("test_repo").join("ref_count").exists());
        let key = match get_content_mgmt_key("test_repo") {
            Ok(cmk) => cmk,
            Err(err) => panic!("get key: {:?}", err),
        };
        {
            // check token file path works as expected
            let token_file_path = key.token_content_file_path("AAGH");
            let expected_tfp = temp_dir.path().join("data").join("ergibus").join("repos").join("test_repo").join("AAG").join("H");
            assert!(token_file_path == expected_tfp);
        }
        {
            let cm = match key.open_content_manager(true) {
                Ok(content_manager) => content_manager,
                Err(err) => panic!("open cm: {:?}", err),
            };
            for i in 1..5 {
                let token = match cm.store_file_contents(&PathBuf::from("./src/content.rs")) {
                    Ok((tkn, _, _)) => tkn,
                    Err(err) => panic!("sfc: {:?}", err),
                };
                match cm.get_ref_count_for_token(&token) {
                    Ok(count) => assert!(count == i),
                    Err(err) => panic!("get ref count #{:?}: {:?}", i, err)
                };
            };
            for i in 1..5 {
                let token = match cm.store_file_contents(&PathBuf::from("./src/snapshot.rs")) {
                    Ok((tkn, _, _)) => tkn,
                    Err(err) => panic!("sfc: {:?}", err),
                };
                match cm.get_ref_count_for_token(&token) {
                    Ok(count) => assert!(count == i),
                    Err(err) => panic!("get ref count #{:?}: {:?}", i, err)
                };
            };
        }
        {
            if let Err(err) = key.open_content_manager(true) {
                panic!("reread: {:?}", err);
            };
        }
        {
            let _cm1 = match key.open_content_manager(false) {
                Ok(content_manager) => content_manager,
                Err(err) => panic!("open cm non exclusive: {:?}", err),
            };
            let _cm2 = match key.open_content_manager(false) {
                Ok(content_manager) => content_manager,
                Err(err) => panic!("open second cm non exclusive: {:?}", err),
            };
        }
        if let Err(err) = temp_dir.close() {
            panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
        };
        if let Err(err) = file.unlock() {
            panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
        };
    }
}
