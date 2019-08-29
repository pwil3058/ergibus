use std::path::{Path, PathBuf};

pub trait RealPathBufType {
    fn is_real_dir(&self) -> bool;
    fn is_symlink_to_dir(&self) -> bool;
    fn is_real_file(&self) -> bool;
    fn is_symlink_to_file(&self) -> bool;
    fn is_symlink(&self) -> bool;
}

macro_rules! impl_real_path_buf_type {
    ( $ptype:ident ) => {
        impl RealPathBufType for $ptype {
            fn is_real_dir(&self) -> bool {
                if let Ok(md) = self.symlink_metadata() {
                    md.is_dir()
                } else {
                    false
                }
            }

            fn is_symlink_to_dir(&self) -> bool {
                if let Ok(md) = self.symlink_metadata() {
                    if md.file_type().is_symlink() {
                        return self.is_dir();
                    }
                };
                false
            }
            fn is_real_file(&self) -> bool {
                if let Ok(md) = self.symlink_metadata() {
                    md.is_file()
                } else {
                    false
                }
            }

            fn is_symlink_to_file(&self) -> bool {
                if let Ok(md) = self.symlink_metadata() {
                    if md.file_type().is_symlink() {
                        return self.is_file();
                    }
                };
                false
            }

            fn is_symlink(&self) -> bool {
                if let Ok(md) = self.symlink_metadata() {
                    return md.file_type().is_symlink();
                };
                false
            }
        }
    };
}

impl_real_path_buf_type!(PathBuf);
impl_real_path_buf_type!(Path);

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn path_buf_is_real_dir_works() {
        assert!(PathBuf::from("src").is_real_dir());
        assert!(!PathBuf::from("nonexistent").is_real_dir());
    }

    #[test]
    fn path_buf_is_symlink_to_dir_works() -> std::result::Result<(), std::io::Error> {
        assert!(!PathBuf::from("src").is_symlink_to_dir());
        assert!(!PathBuf::from("src").is_symlink());
        assert!(!PathBuf::from("nonexistent").is_symlink_to_dir());
        assert!(!PathBuf::from("nonexistent").is_symlink());
        fs::soft_link("target", "link_to_target")?;
        assert!(PathBuf::from("link_to_target").is_symlink_to_dir());
        assert!(PathBuf::from("link_to_target").is_symlink());
        assert!(!PathBuf::from("link_to_target").is_symlink_to_file());
        fs::remove_file(PathBuf::from("link_to_target"))
    }

    #[test]
    fn path_buf_is_real_file_works() {
        assert!(PathBuf::from("COPYRIGHT").is_real_file());
        assert!(!PathBuf::from("nonexistent").is_real_file());
    }

    #[test]
    fn path_buf_is_symlink_to_file_works() -> std::result::Result<(), std::io::Error> {
        assert!(!PathBuf::from("COPYRIGHT").is_symlink_to_file());
        assert!(!PathBuf::from("nonexistent").is_symlink_to_file());
        fs::soft_link("COPYRIGHT", "link_to_COPYRIGHT_2")?;
        assert!(PathBuf::from("link_to_COPYRIGHT_2").is_symlink_to_file());
        assert!(PathBuf::from("link_to_COPYRIGHT_2").is_symlink());
        assert!(!PathBuf::from("link_to_COPYRIGHT_2").is_symlink_to_dir());
        fs::remove_file(PathBuf::from("link_to_COPYRIGHT_2"))
    }

    #[test]
    fn path_is_real_dir_works() {
        assert!(Path::new("src").is_real_dir());
        assert!(!Path::new("nonexistent").is_real_dir());
    }

    #[test]
    fn path_is_symlink_to_dir_works() -> std::result::Result<(), std::io::Error> {
        assert!(!Path::new("src").is_symlink_to_dir());
        assert!(!Path::new("nonexistent").is_symlink_to_dir());
        fs::soft_link("target", "link_to_target_2")?;
        assert!(Path::new("link_to_target_2").is_symlink_to_dir());
        fs::remove_file(Path::new("link_to_target_2"))
    }

    #[test]
    fn path_is_real_file_works() {
        assert!(Path::new("COPYRIGHT").is_real_file());
        assert!(!Path::new("nonexistent").is_real_file());
    }

    #[test]
    fn path_is_symlink_to_file_works() -> std::result::Result<(), std::io::Error> {
        assert!(!Path::new("COPYRIGHT").is_symlink_to_file());
        assert!(!Path::new("nonexistent").is_symlink_to_file());
        fs::soft_link("COPYRIGHT", "link_to_COPYRIGHT")?;
        assert!(Path::new("link_to_COPYRIGHT").is_symlink_to_file());
        fs::remove_file(Path::new("link_to_COPYRIGHT"))
    }
}
