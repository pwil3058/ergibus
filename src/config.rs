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

use std::env;
use std::path::PathBuf;

use pw_pathux;

const DEFAULT_CONFIG_DIR_PATH: &str = "~/.config/ergibus";

const DCDP_OVERRIDE_ENVAR: &str = "ERGIBUS_CONFIG_DIR";

pub fn abs_default_config_dir_path() -> PathBuf {
    match pw_pathux::expand_home_dir(&PathBuf::from(DEFAULT_CONFIG_DIR_PATH)) {
        Some(expanded_dir) => expanded_dir,
        None => panic!("{:?}: line {:?}: config dir path expansion failed", file!(), line!())
    }
}

fn get_config_dir_path() -> PathBuf {
    match env::var(DCDP_OVERRIDE_ENVAR) {
        Ok(dir_path) => if dir_path.len() == 0 {
            abs_default_config_dir_path()
        } else if dir_path.starts_with("~") {
            match pw_pathux::expand_home_dir(&PathBuf::from(dir_path)) {
                Some(expanded_dir) => expanded_dir,
                None => panic!("{:?}: line {:?}: config dir path expansion failed", file!(), line!())
            }
        } else {
            PathBuf::from(dir_path)
        },
        Err(_) => abs_default_config_dir_path()
    }
}

pub fn get_archive_config_dir_path() -> PathBuf {
    get_config_dir_path().join("archives")
}

pub fn get_gui_config_dir_path() -> PathBuf {
    get_config_dir_path().join("gui")
}

pub fn get_repo_config_dir_path() -> PathBuf {
    get_config_dir_path().join("repos")
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_config_dir_works() {
        let new_path = "./TEST/config";
        env::set_var(DCDP_OVERRIDE_ENVAR, new_path);
        assert_eq!(get_config_dir_path(), PathBuf::from(new_path));
        assert_eq!(get_archive_config_dir_path(), PathBuf::from(new_path).join("archives"));
        assert_eq!(get_repo_config_dir_path(), PathBuf::from(new_path).join("repos"));
        env::set_var(DCDP_OVERRIDE_ENVAR, "");
        assert_eq!(get_config_dir_path(), abs_default_config_dir_path());
        assert_eq!(get_archive_config_dir_path(), abs_default_config_dir_path().join("archives"));
        assert_eq!(get_repo_config_dir_path(), abs_default_config_dir_path().join("repos"));
    }
}
