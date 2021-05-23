// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use dirs;

use std::env;
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

pub fn first_subpath_as_os_string(path: &Path) -> Option<OsString> {
    for c in path.components() {
        match c {
            Component::RootDir => continue,
            Component::Normal(component) => {
                return Some(component.to_os_string());
            }
            Component::Prefix(_) => panic!("Not implemented for Windows"),
            Component::ParentDir => panic!("Illegal component"),
            _ => (),
        }
    }
    None
}

pub fn expand_home_dir(path: &Path) -> Option<PathBuf> {
    if path.is_absolute() {
        return Some(path.to_path_buf());
    } else if !path.exists() {
        let mut components = path.components();
        if let Some(first_component) = components.next() {
            if let Component::Normal(text) = first_component {
                if text == "~" {
                    if let Some(home_dir_path) = dirs::home_dir() {
                        return Some(home_dir_path.join(components.as_path()));
                    }
                }
            }
        }
    };
    None
}

pub fn absolute_path_buf(path: &Path) -> PathBuf {
    if path.is_relative() {
        if let Ok(current_dir_path) = env::current_dir() {
            let mut components = path.components();
            if let Some(first_component) = components.next() {
                if let Component::CurDir = first_component {
                    return current_dir_path.join(components.as_path());
                } else {
                    return current_dir_path.join(path);
                }
            } else {
                return current_dir_path;
            }
        } else {
            panic!(
                "File: {} Line: {} : can't find current directory???",
                file!(),
                line!()
            )
        }
    };
    path.to_path_buf()
}
