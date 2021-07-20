// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>
use std::{
    env, io,
    path::{Component, Path, PathBuf, StripPrefixError},
};

use dirs;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Could not find user's home directory.")]
    CouldNotFindHome,
    #[error("Could not find current directory's parent.")]
    CouldNotFindParent,
    #[error("I/O Error")]
    IOError(#[from] io::Error),
    #[error("Error stripping path's prefix")]
    StripPrefixError(#[from] StripPrefixError),
    #[error("Unexpected prefix for this operation.")]
    UnexpectedPrefix,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PathType {
    Absolute,
    RelativeCurDir,
    RelativeCurDirImplicit,
    RelativeParentDir,
    RelativeHomeDir,
    Empty,
}

impl PathType {
    pub fn of<P: AsRef<Path>>(path_arg: P) -> Self {
        let path = path_arg.as_ref();
        match path.components().next() {
            None => PathType::Empty,
            Some(component) => match component {
                Component::RootDir | Component::Prefix(_) => PathType::Absolute,
                Component::CurDir => PathType::RelativeCurDir,
                Component::ParentDir => PathType::RelativeParentDir,
                Component::Normal(os_string) => {
                    if os_string == "~" {
                        PathType::RelativeHomeDir
                    } else {
                        PathType::RelativeCurDirImplicit
                    }
                }
            },
        }
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
    match PathType::of(path) {
        PathType::Absolute => Ok(path.to_path_buf()),
        PathType::RelativeCurDir => expand_current_dir(path),
        PathType::RelativeParentDir => expand_parent_dir(path),
        PathType::RelativeHomeDir => expand_home_dir(path),
        PathType::RelativeCurDirImplicit => prepend_current_dir(path),
        PathType::Empty => Ok(env::current_dir()?),
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
