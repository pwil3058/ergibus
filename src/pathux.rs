use std::path::{Path, PathBuf, Component};

pub fn split_abs_path(abs_path: &Path) -> Vec<String> {
    assert!(abs_path.is_absolute());
    let mut vec: Vec<String> = Vec::new();
    for c in abs_path.components() {
        match c {
            Component::Normal(component) => {
                let oss = component.to_os_string();
                vec.push(oss.into_string().unwrap());
            },
            Component::Prefix(_) => panic!("Not implemented for Windows"),
            Component::ParentDir => panic!("Illegal component"),
            _ => ()
        }
    }
    vec
}

pub fn split_rel_path(rel_path: &Path) -> Vec<String> {
    assert!(rel_path.is_relative());
    let mut vec: Vec<String> = Vec::new();
    for c in rel_path.components() {
        match c {
            Component::Normal(component) => {
                let oss = component.to_os_string();
                vec.push(oss.into_string().unwrap());
            },
            Component::Prefix(_) => panic!("Not implemented for Windows"),
            Component::ParentDir => panic!("Illegal component"),
            _ => ()
        }
    }
    vec
}

pub fn first_subpath_as_string(path: &Path) -> Option<String> {
    for c in path.components() {
        match c {
            Component::RootDir => continue,
            Component::Normal(component) => {
                let oss = component.to_os_string();
                return Some(oss.into_string().unwrap());
            },
            Component::Prefix(_) => panic!("Not implemented for Windows"),
            Component::ParentDir => panic!("Illegal component"),
            _ => ()
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_abs_path_works() {
        let parts = split_abs_path(Path::new("/home/peter/SCR"));
        assert_eq!(parts, vec!["home", "peter", "SCR"]);
    }

    #[test]
    #[should_panic]
    fn split_abs_path_panics() {
        let parts = split_abs_path(Path::new("/home/../peter/SCR"));
        assert_eq!(parts, vec!["home", "peter", "SCR"]);
    }

    #[test]
    fn first_subpath_as_string_works() {
        assert_eq!(Some("first".to_string()), first_subpath_as_string(Path::new("first/second")));
        assert_ne!(Some("second".to_string()), first_subpath_as_string(Path::new("first/second")));
        assert_eq!(Some("first".to_string()), first_subpath_as_string(Path::new("/first/second")));
    }
}
