use std::env;
use std::path::PathBuf;

use dirs;

use path_ext;

const DEFAULT_CONFIG_DIR_PATH: &str = "~/.config/dychatat";

const DCDP_OVERRIDE_ENVAR: &str = "DYCHATAT_CONFIG_DIR";

pub fn abs_default_config_dir_path() -> PathBuf {
    match dirs::config_dir() {
        Some(config_dir) => config_dir.join("dychatat"),
        None => match path_ext::expand_home_dir(&PathBuf::from(DEFAULT_CONFIG_DIR_PATH)) {
            Ok(expanded_dir) => expanded_dir,
            Err(_) => panic!("config dir path expansion failed"),
        },
    }
}

fn get_config_dir_path() -> PathBuf {
    match env::var(DCDP_OVERRIDE_ENVAR) {
        Ok(dir_path) => {
            if dir_path.len() == 0 {
                abs_default_config_dir_path()
            } else if dir_path.starts_with("~") {
                match path_ext::expand_home_dir(&PathBuf::from(dir_path)) {
                    Ok(expanded_dir) => expanded_dir,
                    Err(_) => panic!("config dir path expansion failed",),
                }
            } else {
                PathBuf::from(dir_path)
            }
        }
        Err(_) => abs_default_config_dir_path(),
    }
}

pub fn _get_gui_config_dir_path() -> PathBuf {
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
        assert_eq!(
            get_repo_config_dir_path(),
            PathBuf::from(new_path).join("repos")
        );
        env::set_var(DCDP_OVERRIDE_ENVAR, "");
        assert_eq!(get_config_dir_path(), abs_default_config_dir_path());
        assert_eq!(
            get_repo_config_dir_path(),
            abs_default_config_dir_path().join("repos")
        );
    }
}
