// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au> <pwil3058@outlook.com>
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::UnreferencedContentData;
pub use crate::{ContentManager, ContentMgmtKey, HashAlgorithm, Mutability, RepoSpec};

use crate::config;
use crate::{RepoError, RepoResult};

pub fn content_repo_exists(repo_name: &str) -> bool {
    get_repo_spec_file_path(repo_name).exists()
}

pub fn get_content_mgmt_key(repo_name: &str) -> RepoResult<ContentMgmtKey> {
    if !content_repo_exists(repo_name) {
        Err(RepoError::UnknownRepo(repo_name.to_string()))
    } else {
        let spec = read_repo_spec(repo_name)?;
        Ok(ContentMgmtKey::from(&spec))
    }
}

pub fn create_new_repo<P: AsRef<Path>>(
    name: &str,
    location: P,
    hash_algortithm_str: &str,
) -> RepoResult<()> {
    if content_repo_exists(name) {
        return Err(RepoError::RepoExists(name.to_string()));
    }

    let hash_algorithm = HashAlgorithm::from_str(hash_algortithm_str)?;

    let mut repo_dir_path = location.as_ref().to_path_buf();
    repo_dir_path.push("dychatat");
    repo_dir_path.push("repos");
    repo_dir_path.push(name);

    let spec = RepoSpec::new(repo_dir_path, hash_algorithm);

    ContentMgmtKey::from(&spec).create_repo_dir()?;

    write_repo_spec(name, &spec)?;
    Ok(())
}

fn get_repo_spec_file_path(repo_name: &str) -> PathBuf {
    config::get_repo_config_dir_path().join(repo_name)
}

pub fn read_repo_spec(repo_name: &str) -> RepoResult<RepoSpec> {
    let spec_file_path = get_repo_spec_file_path(repo_name);
    let spec_file = File::open(&spec_file_path)?;
    // .map_err(|err| RepoError::RepoReadError(err, spec_file_path.clone()))?;
    let spec = RepoSpec::from_reader(spec_file)?;
    Ok(spec)
}

fn write_repo_spec(repo_name: &str, repo_spec: &RepoSpec) -> RepoResult<()> {
    let spec_file_path = get_repo_spec_file_path(repo_name);
    if spec_file_path.exists() {
        return Err(RepoError::RepoExists(repo_name.to_string()));
    }
    match spec_file_path.parent() {
        Some(config_dir_path) => {
            if !config_dir_path.exists() {
                fs::create_dir_all(&config_dir_path)?;
                // .map_err(|err| RepoError::RepoWriteError(err, config_dir_path.to_path_buf()))?;
            }
        }
        None => (),
    }
    let spec_file = File::create(&spec_file_path)?;
    // .map_err(|err| RepoError::RepoWriteError(err, spec_file_path.clone()))?;
    repo_spec.to_writer(spec_file)?;
    Ok(())
}

pub fn get_repo_names() -> Vec<String> {
    let mut names = Vec::new();
    if let Ok(dir_entries) = fs::read_dir(config::get_repo_config_dir_path()) {
        for entry_or_err in dir_entries {
            if let Ok(entry) = entry_or_err {
                let path = entry.path();
                if path.is_file() {
                    if let Some(file_name) = path.file_name() {
                        if let Some(file_name) = file_name.to_str() {
                            names.push(file_name.to_string());
                        }
                    }
                }
            }
        }
    };
    names
}

pub fn delete_repository(repo_name: &str) -> RepoResult<()> {
    let repo_key = get_content_mgmt_key(repo_name)?;
    let content_manager = repo_key.open_content_manager(Mutability::Mutable)?;
    content_manager.delete()?;
    let repo_spec_path = get_repo_spec_file_path(repo_name);
    fs::remove_file(repo_spec_path)?;
    Ok(())
}

pub fn prune_repository(repo_name: &str) -> RepoResult<UnreferencedContentData> {
    let repo_key = get_content_mgmt_key(repo_name)?;
    let content_manager = repo_key.open_content_manager(Mutability::Mutable)?;
    Ok(content_manager.prune_contents()?)
}

#[cfg(test)]
mod content_tests {
    use super::*;
    use crate::Mutability;
    use fs2::FileExt;
    use std::env;
    use std::fs::OpenOptions;
    use tempdir::TempDir;

    #[test]
    fn repo_works() {
        let file = OpenOptions::new()
            .write(true)
            .open("../test_lock_file")
            .unwrap();
        assert!(file.lock_exclusive().is_ok());

        let temp_dir = TempDir::new("REPO_TEST").unwrap();

        env::set_var("DYCHATAT_CONFIG_DIR", temp_dir.path().join("config"));
        let data_dir = temp_dir.path().join("data");
        let data_dir_str = data_dir.to_str().unwrap();
        assert!(create_new_repo("test_repo", data_dir_str, "Sha1").is_ok());
        assert!(temp_dir
            .path()
            .join("config")
            .join("repos")
            .join("test_repo")
            .exists());
        assert!(temp_dir
            .path()
            .join("data")
            .join("dychatat")
            .join("repos")
            .join("test_repo")
            .join("ref_count")
            .exists());
        let key = get_content_mgmt_key("test_repo").unwrap();
        {
            let cm = key.open_content_manager(Mutability::Mutable).unwrap();
            for i in 1..5 {
                let mut file = File::open("./src/content.rs").unwrap();
                let (token, _, _) = cm.store_contents(&mut file).unwrap();
                assert_eq!(cm.ref_count_for_token(&token).unwrap(), i);
            }
            for i in 1..5 {
                let mut file = File::open("./src/error.rs").unwrap();
                let (token, _, _) = cm.store_contents(&mut file).unwrap();
                assert_eq!(cm.ref_count_for_token(&token).unwrap(), i);
            }
        }
        {
            assert!(key.open_content_manager(Mutability::Mutable).is_ok())
        }
        {
            let _cm1 = key.open_content_manager(Mutability::Immutable).unwrap();
            let _cm2 = key.open_content_manager(Mutability::Immutable).unwrap();
        }
        assert!(temp_dir.close().is_ok());
        assert!(file.unlock().is_ok());
    }
}
