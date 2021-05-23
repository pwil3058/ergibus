// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>
use std::{
    env, io,
    path::{Component, Path, PathBuf, StripPrefixError},
};

use dirs;

#[derive(Debug)]
pub enum Error {
    CouldNotFindHome,
    CouldNotFindParent,
    IOError(io::Error),
    StripPrefixError(StripPrefixError),
    UnexpectedPrefix,
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::IOError(error)
    }
}

impl From<StripPrefixError> for Error {
    fn from(error: StripPrefixError) -> Self {
        Error::StripPrefixError(error)
    }
}

pub fn expand_current_dir<P: AsRef<Path>>(path_arg: P) -> Result<PathBuf, Error> {
    let path = path_arg.as_ref();
    if path.starts_with(Component::CurDir) {
        let cur_dir = env::current_dir()?;
        let path_tail = path.strip_prefix(Component::CurDir)?;
        Ok(cur_dir.join(path_tail))
    } else {
        Err(Error::UnexpectedPrefix)
    }
}

pub fn expand_parent_dir<P: AsRef<Path>>(path_arg: P) -> Result<PathBuf, Error> {
    let path = path_arg.as_ref();
    if path.starts_with(Component::ParentDir) {
        let cur_dir = env::current_dir()?;
        let parent_dir = match cur_dir.parent() {
            Some(parent_dir) => parent_dir,
            None => return Err(Error::CouldNotFindParent),
        };
        let path_tail = path.strip_prefix(Component::ParentDir)?;
        Ok(parent_dir.join(path_tail))
    } else {
        Err(Error::UnexpectedPrefix)
    }
}

pub fn expand_home_dir<P: AsRef<Path>>(path_arg: P) -> Result<PathBuf, Error> {
    let path = path_arg.as_ref();
    if path.starts_with("~") {
        let home_dir = match dirs::home_dir() {
            Some(home_dir) => home_dir,
            None => return Err(Error::CouldNotFindHome),
        };
        let path_tail = path.strip_prefix("~")?;
        Ok(home_dir.join(path_tail))
    } else {
        Err(Error::UnexpectedPrefix)
    }
}

pub fn prepend_current_dir<P: AsRef<Path>>(path_arg: P) -> Result<PathBuf, Error> {
    let path = path_arg.as_ref();
    match path.components().next() {
        None => Ok(env::current_dir()?),
        Some(component) => match component {
            Component::Normal(os_string) => {
                if os_string == "~" {
                    Err(Error::UnexpectedPrefix)
                } else {
                    let cur_dir = env::current_dir()?;
                    Ok(cur_dir.join(path))
                }
            }
            _ => Err(Error::UnexpectedPrefix),
        },
    }
}

pub fn absolute_path_buf<P: AsRef<Path>>(path_arg: P) -> Result<PathBuf, Error> {
    let path = path_arg.as_ref();
    match path.components().next() {
        None => Ok(env::current_dir()?),
        Some(component) => match component {
            Component::RootDir | Component::Prefix(_) => Ok(path.to_path_buf()),
            Component::CurDir => expand_current_dir(path),
            Component::ParentDir => expand_parent_dir(path),
            Component::Normal(os_string) => {
                if os_string == "~" {
                    expand_home_dir(path)
                } else {
                    prepend_current_dir(path)
                }
            }
        },
    }
}

#[cfg(test)]
mod path_ext_tests {
    use crate::{
        absolute_path_buf, expand_current_dir, expand_home_dir, expand_parent_dir,
        prepend_current_dir,
    };
    use std::env;

    #[test]
    fn home_path_expansions() {
        let home_dir = dirs::home_dir().unwrap();
        assert!(expand_home_dir("/home/dir").is_err());
        assert_eq!(
            expand_home_dir("~/whatever").unwrap(),
            home_dir.join("whatever")
        );
        assert_eq!(
            absolute_path_buf("~/whatever").unwrap(),
            home_dir.join("whatever")
        );
    }

    #[test]
    fn cur_path_expansions() {
        let cur_dir = env::current_dir().unwrap();
        assert!(expand_current_dir("/home/dir").is_err());
        assert_eq!(
            expand_current_dir("./whatever").unwrap(),
            cur_dir.join("whatever")
        );
        assert_eq!(
            absolute_path_buf("./whatever").unwrap(),
            cur_dir.join("whatever")
        );
        assert!(prepend_current_dir("/home/dir").is_err());
        assert_eq!(
            prepend_current_dir("whatever").unwrap(),
            cur_dir.join("whatever")
        );
        assert_eq!(
            absolute_path_buf("whatever").unwrap(),
            cur_dir.join("whatever")
        );
    }

    #[test]
    fn parent_path_expansions() {
        let cur_dir = env::current_dir().unwrap();
        let parent_dir = cur_dir.parent().unwrap();
        assert!(expand_parent_dir("/home/dir").is_err());
        assert_eq!(
            expand_parent_dir("../whatever").unwrap(),
            parent_dir.join("whatever")
        );
        assert_eq!(
            absolute_path_buf("../whatever").unwrap(),
            parent_dir.join("whatever")
        );
    }
}
