use std::fs::{self, File};
use std::path::PathBuf;
use std::str::FromStr;

pub use dychatat::{ContentManager, ContentMgmtKey, HashAlgorithm, RepoSpec};

use crate::config;
use crate::eerror::{EError, EResult};

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
        return Err(EError::RepoExists(name.to_string()));
    }

    let hash_algorithm = HashAlgorithm::from_str(hash_algortithm_str)?;

    let mut repo_dir_path = PathBuf::from(location);
    repo_dir_path.push("ergibus");
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

fn read_repo_spec(repo_name: &str) -> EResult<RepoSpec> {
    let spec_file_path = get_repo_spec_file_path(repo_name);
    let spec_file = File::open(&spec_file_path)
        .map_err(|err| EError::RepoReadError(err, spec_file_path.clone()))?;
    let spec = RepoSpec::from_reader(spec_file)?;
    Ok(spec)
}

fn write_repo_spec(repo_name: &str, repo_spec: &RepoSpec) -> EResult<()> {
    let spec_file_path = get_repo_spec_file_path(repo_name);
    if spec_file_path.exists() {
        return Err(EError::RepoExists(repo_name.to_string()));
    }
    match spec_file_path.parent() {
        Some(config_dir_path) => {
            if !config_dir_path.exists() {
                fs::create_dir_all(&config_dir_path)
                    .map_err(|err| EError::RepoWriteError(err, config_dir_path.to_path_buf()))?;
            }
        }
        None => (),
    }
    let spec_file = File::create(&spec_file_path)
        .map_err(|err| EError::RepoWriteError(err, spec_file_path.clone()))?;
    repo_spec.to_writer(spec_file)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dychatat::Mutability;
    use fs2::FileExt;
    use std::env;
    use std::fs::OpenOptions;
    use tempdir::TempDir;

    #[test]
    fn repo_works() {
        let file = OpenOptions::new()
            .write(true)
            .open("./test_lock_file")
            .unwrap_or_else(|err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err));
        if let Err(err) = file.lock_exclusive() {
            panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
        };
        let temp_dir = TempDir::new("REPO_TEST")
            .unwrap_or_else(|err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err));
        env::set_var("ERGIBUS_CONFIG_DIR", temp_dir.path().join("config"));
        let data_dir = temp_dir.path().join("data");
        let data_dir_str = match data_dir.to_str() {
            Some(data_dir_str) => data_dir_str,
            None => panic!("{:?}: line {:?}", file!(), line!()),
        };
        if let Err(err) = create_new_repo("test_repo", data_dir_str, "Sha1") {
            panic!("new repo: {:?}", err);
        }
        assert!(temp_dir
            .path()
            .join("config")
            .join("repos")
            .join("test_repo")
            .exists());
        assert!(temp_dir
            .path()
            .join("data")
            .join("ergibus")
            .join("repos")
            .join("test_repo")
            .join("ref_count")
            .exists());
        let key = match get_content_mgmt_key("test_repo") {
            Ok(cmk) => cmk,
            Err(err) => panic!("get key: {:?}", err),
        };
        {
            let cm = match key.open_content_manager(Mutability::Mutable) {
                Ok(content_manager) => content_manager,
                Err(err) => panic!("open cm: {:?}", err),
            };
            for i in 1..5 {
                let mut file = File::open("./src/content.rs").unwrap();
                let token = match cm.store_contents(&mut file) {
                    Ok((tkn, _, _)) => tkn,
                    Err(err) => panic!("sfc: {:?}", err),
                };
                match cm.ref_count_for_token(&token) {
                    Ok(count) => assert!(count == i),
                    Err(err) => panic!("get ref count #{:?}: {:?}", i, err),
                };
            }
            for i in 1..5 {
                let mut file = File::open("./src/snapshot.rs").unwrap();
                let token = match cm.store_contents(&mut file) {
                    Ok((tkn, _, _)) => tkn,
                    Err(err) => panic!("sfc: {:?}", err),
                };
                match cm.ref_count_for_token(&token) {
                    Ok(count) => assert!(count == i),
                    Err(err) => panic!("get ref count #{:?}: {:?}", i, err),
                };
            }
        }
        {
            if let Err(err) = key.open_content_manager(Mutability::Mutable) {
                panic!("reread: {:?}", err);
            };
        }
        {
            let _cm1 = match key.open_content_manager(Mutability::Immutable) {
                Ok(content_manager) => content_manager,
                Err(err) => panic!("open cm non exclusive: {:?}", err),
            };
            let _cm2 = match key.open_content_manager(Mutability::Immutable) {
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
