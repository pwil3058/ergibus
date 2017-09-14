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

use std::cell::Cell;
use std::fs::{self, File};
use std::io::prelude::*;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use hex::ToHex;

use crypto_hash;
use serde_yaml;

use config;
use eerror::{EError, EResult};

pub fn content_repo_exists(repo_name: &str) -> bool {
    repo_name == "dummy"
}

pub fn get_content_mgmt_key(repo_name: &str) -> EResult<ContentMgmtKey> {
    if repo_name == "dummy" {
        Ok(ContentMgmtKey::new_dummy())
    } else {
        Err(EError::UnknownRepo(repo_name.to_string()))
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
    // TODO: create lock file and reference count file
    let spec = RepoSpec {
        base_dir_path: repo_dir_path,
        hash_algorithm: hash_algorithm
    };
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
    let config_dir_path = config::get_repo_config_dir_path();
    let mut spec_file_path = config_dir_path.join(repo_name);
    spec_file_path.set_extension("rspec");
    spec_file_path
}

fn read_repo_spec(repo_name: &str) -> EResult<RepoSpec> {
    let mut spec_file_path = get_repo_spec_file_path(repo_name);
    let mut spec_file = File::open(&spec_file_path).map_err(|err| EError::RepoReadError(err, spec_file_path.clone()))?;
    let spec: RepoSpec = serde_yaml::from_reader(&spec_file).map_err(|err| EError::RepoYamlReadError(err, repo_name.to_string()))?;
    Ok(spec)
}

fn write_repo_spec(repo_name: &str, repo_spec: &RepoSpec) -> EResult<()> {
    let mut spec_file_path = get_repo_spec_file_path(repo_name);
    if spec_file_path.exists() {
        return Err(EError::RepoExists(repo_name.to_string()))
    }
    match spec_file_path.parent() {
        Some(config_dir_path) => if !config_dir_path.exists() {
            fs::create_dir_all(&config_dir_path).map_err(|err| EError::RepoWriteError(err, config_dir_path.to_path_buf()))?;
        },
        None => (),
    }
    let mut spec_file = File::create(&spec_file_path).map_err(|err| EError::RepoWriteError(err, spec_file_path.clone()))?;
    serde_yaml::to_writer(&spec_file, repo_spec).map_err(|err| EError::RepoYamlWriteError(err, repo_name.to_string()))?;
    Ok(())
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
        HashAlgorithm::Sha1 => crypto_hash::Hasher::new(crypto_hash::Algorithm::SHA1),
        HashAlgorithm::Sha256 => crypto_hash::Hasher::new(crypto_hash::Algorithm::SHA256),
        HashAlgorithm::Sha512 => crypto_hash::Hasher::new(crypto_hash::Algorithm::SHA512),
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

    pub fn store_file_contents(&self, abs_file_path: &Path) -> EResult<String> {
        self.count.replace(self.count.get() + 1);
        let mut file = File::open(abs_file_path).map_err(|err| EError::ContentStoreIOError(err))?;
        let digest = file_digest(self.content_mgmt_key.hash_algortithm, &mut file).map_err(|err| EError::ContentStoreIOError(err))?;
        Ok(digest)
    }

    pub fn release_contents(&self, content_token: &str) -> EResult<()> {
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
