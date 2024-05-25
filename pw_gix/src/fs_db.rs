// Copyright 2019 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

//! File system database of the current directory to feed file tree
//! stores/views

use std::rc::Rc;

use gtk::TreeIter; //{StaticType, ToValue, TreeIter};

use pw_pathux::UsableDirEntry;

pub use crate::glibx::*;
pub use crate::gtkx::tree_store::TreeRowOps;
pub use crate::gtkx::value::Row;

pub trait FsObjectIfce {
    fn new(name: &str, path: &str, is_dir: bool) -> Self;
    fn from_dir_entry(dir_entry: &UsableDirEntry) -> Self;

    fn tree_store_spec() -> Vec<glib::Type>;
    fn tree_view_columns() -> Vec<gtk::TreeViewColumn>;

    fn row_is_a_dir<S: TreeRowOps>(store: &S, iter: &TreeIter) -> bool;
    fn row_is_place_holder<S: TreeRowOps>(store: &S, iter: &TreeIter) -> bool;
    fn get_name_from_row<S: TreeRowOps>(store: &S, iter: &TreeIter) -> String;
    fn get_path_from_row<S: TreeRowOps>(store: &S, iter: &TreeIter) -> String;
    fn set_place_holder_values<S: TreeRowOps>(store: &S, iter: &TreeIter);

    fn update_row_if_required<S: TreeRowOps>(&self, store: &S, iter: &TreeIter) -> bool;
    fn set_row_values<S: TreeRowOps>(&self, store: &S, iter: &TreeIter);

    fn name(&self) -> &str;
    fn path(&self) -> &str;
    fn is_dir(&self) -> bool;
}

pub trait FsDbIfce<FSOI>
where
    FSOI: FsObjectIfce,
{
    fn honours_hide_clean() -> bool;
    fn honours_show_hidden() -> bool;

    fn new() -> Self;

    fn dir_contents(
        &self,
        dir_path: &str,
        show_hidden: bool,
        hide_clean: bool,
    ) -> (Rc<Vec<FSOI>>, Rc<Vec<FSOI>>);

    fn update_if_necessary(&self) -> bool {
        true
    }

    fn reset(&self);
}

// Plain OS FS Database
// This is done via macro so that it can be used outside the crate
#[macro_export]
macro_rules! impl_os_fs_db {
    ( $db:ident, $db_dir:ident ) => {
        #[derive(Debug)]
        struct $db_dir<FSOI>
        where
            FSOI: FsObjectIfce,
        {
            path: String,
            show_hidden: bool,
            hide_clean: bool,
            dirs_data: Rc<Vec<FSOI>>,
            files_data: Rc<Vec<FSOI>>,
            hash_digest: Option<Vec<u8>>,
            sub_dirs: HashMap<String, $db_dir<FSOI>>,
        }

        impl<FSOI> $db_dir<FSOI>
        where
            FSOI: FsObjectIfce,
        {
            fn new(dir_path: &str, show_hidden: bool, hide_clean: bool) -> Self {
                Self {
                    path: dir_path.to_string(),
                    show_hidden,
                    hide_clean,
                    dirs_data: Rc::new(vec![]),
                    files_data: Rc::new(vec![]),
                    hash_digest: None,
                    sub_dirs: HashMap::new(),
                }
            }

            fn current_hash_digest(&self) -> Vec<u8> {
                let mut hasher = Hasher::new(Algorithm::SHA256);
                if let Ok(dir_entries) = UsableDirEntry::get_entries(&self.path) {
                    for dir_entry in dir_entries {
                        let path = dir_entry.path().to_string_lossy().into_owned();
                        hasher.write_all(&path.into_bytes()).unwrap()
                    }
                }
                hasher.finish()
            }

            fn is_current(&self) -> bool {
                match self.hash_digest {
                    None => return true,
                    Some(ref hash_digest) => {
                        if *hash_digest != self.current_hash_digest() {
                            return false;
                        } else {
                            for sub_dir in self.sub_dirs.values() {
                                if !sub_dir.is_current() {
                                    return false;
                                }
                            }
                        }
                    }
                }
                true
            }

            fn populate(&mut self) {
                let mut hasher = Hasher::new(Algorithm::SHA256);
                if let Ok(dir_entries) = UsableDirEntry::get_entries(&self.path) {
                    let mut dirs = vec![];
                    let mut files = vec![];
                    for dir_entry in dir_entries {
                        let path = dir_entry.path().to_string_lossy().into_owned();
                        hasher.write_all(&path.into_bytes()).unwrap();
                        if !self.show_hidden && dir_entry.file_name().starts_with(".") {
                            continue;
                        }
                        if dir_entry.is_dir() {
                            let path = dir_entry.path().to_string_lossy().into_owned();
                            dirs.push(FSOI::from_dir_entry(&dir_entry));
                            self.sub_dirs.insert(
                                dir_entry.file_name(),
                                $db_dir::<FSOI>::new(&path, self.show_hidden, self.hide_clean),
                            );
                        } else {
                            files.push(FSOI::from_dir_entry(&dir_entry));
                        }
                    }
                    dirs.sort_unstable_by(|a, b| a.name().partial_cmp(b.name()).unwrap());
                    files.sort_unstable_by(|a, b| a.name().partial_cmp(b.name()).unwrap());
                    self.dirs_data = Rc::new(dirs);
                    self.files_data = Rc::new(files);
                }
                self.hash_digest = Some(hasher.finish());
            }

            fn find_dir(&mut self, components: &[StrPathComponent]) -> Option<&mut $db_dir<FSOI>> {
                if self.hash_digest.is_none() {
                    self.populate();
                }
                if components.len() == 0 {
                    Some(self)
                } else {
                    assert!(components[0].is_normal());
                    let name = components[0].to_string();
                    match self.sub_dirs.get_mut(&name) {
                        Some(subdir) => subdir.find_dir(&components[1..]),
                        None => None,
                    }
                }
            }

            fn dirs_and_files(&mut self) -> (Rc<Vec<FSOI>>, Rc<Vec<FSOI>>) {
                (Rc::clone(&self.dirs_data), Rc::clone(&self.files_data))
            }
        }

        pub struct $db<FSOI>
        where
            FSOI: FsObjectIfce,
        {
            base_dir: RefCell<$db_dir<FSOI>>,
            curr_dir: RefCell<String>, // so we can tell if there's a change of current directory
        }

        impl<FSOI> FsDbIfce<FSOI> for $db<FSOI>
        where
            FSOI: FsObjectIfce,
        {
            fn honours_hide_clean() -> bool {
                false
            }

            fn honours_show_hidden() -> bool {
                true
            }

            fn new() -> Self {
                let curr_dir = str_path_current_dir_or_panic();
                let base_dir = $db_dir::<FSOI>::new("./", false, false); // paths are relative
                Self {
                    base_dir: RefCell::new(base_dir),
                    curr_dir: RefCell::new(curr_dir),
                }
            }

            fn dir_contents(
                &self,
                dir_path: &str,
                show_hidden: bool,
                hide_clean: bool,
            ) -> (Rc<Vec<FSOI>>, Rc<Vec<FSOI>>) {
                assert!(dir_path.path_is_relative());
                self.check_visibility(show_hidden, hide_clean);
                let components = dir_path.to_string().path_components();
                assert!(components[0].is_cur_dir());
                if let Some(ref mut dir) = self.base_dir.borrow_mut().find_dir(&components[1..]) {
                    dir.dirs_and_files()
                } else {
                    (Rc::new(vec![]), Rc::new(vec![]))
                }
            }

            fn update_if_necessary(&self) -> bool {
                if self.curr_dir_changed() {
                    self.reset();
                    true
                } else if !self.base_dir.borrow_mut().is_current() {
                    *self.base_dir.borrow_mut() = $db_dir::new("./", false, false);
                    true
                } else {
                    false
                }
            }

            fn reset(&self) {
                *self.curr_dir.borrow_mut() = str_path_current_dir_or_panic();
                *self.base_dir.borrow_mut() = $db_dir::new("./", false, false);
            }
        }

        impl<FSOI> $db<FSOI>
        where
            FSOI: FsObjectIfce,
        {
            fn curr_dir_changed(&self) -> bool {
                *self.curr_dir.borrow() != str_path_current_dir_or_panic()
            }

            fn check_visibility(&self, show_hidden: bool, hide_clean: bool) {
                let mut base_dir = self.base_dir.borrow_mut();
                if base_dir.show_hidden != show_hidden || base_dir.hide_clean != hide_clean {
                    *base_dir = $db_dir::new("./", show_hidden, hide_clean);
                }
            }
        }
    };
}

// Simple OS FS Object
// This is done via macro so that it can be used outside the crate
#[macro_export]
macro_rules! impl_simple_fs_object {
    ( $sfso:ident ) => {
        lazy_static! {
            pub static ref OS_FS_DB_ROW_SPEC: [glib::Type; 4] =
                [
                    glib::Type::String,   // 0 Name
                    glib::Type::String,   // 1 Path
                    glib::Type::String,   // 2 Path
                    bool::static_type(),        // 3 is a directory?
                ];
        }

        const NAME: i32 = 0;
        const PATH: i32 = 1;
        const ICON: i32 = 2;
        const IS_DIR: i32 = 3;

        #[derive(Debug)]
        pub struct $sfso {
            name: String,
            path: String,
            is_dir: bool,
        }

        impl FsObjectIfce for $sfso {
            fn new(name: &str, path: &str, is_dir: bool) -> Self {
                Self {
                    name: name.to_string(),
                    path: path.to_string(),
                    is_dir,
                }
            }

            fn from_dir_entry(dir_entry: &UsableDirEntry) -> Self {
                $sfso {
                    name: dir_entry.file_name(),
                    path: dir_entry.path().to_string_lossy().into_owned(),
                    is_dir: dir_entry.is_dir(),
                }
            }

            fn tree_store_spec() -> Vec<glib::Type> {
                OS_FS_DB_ROW_SPEC.to_vec()
            }

            fn tree_view_columns() -> Vec<gtk::TreeViewColumn> {
                let col = gtk::TreeViewColumn::new();
                let cell = gtk::CellRendererPixbuf::new();
                col.pack_start(&cell, false);
                col.add_attribute(&cell, "icon-name", ICON);
                let cell = gtk::CellRendererText::new();
                cell.set_property_editable(false);
                col.pack_start(&cell, false);
                col.add_attribute(&cell, "text", NAME);
                vec![col]
            }

            fn row_is_a_dir<S: TreeRowOps>(store: &S, iter: &TreeIter) -> bool {
                store.get_value(iter, IS_DIR).get_ok_some::<bool>()
            }

            fn row_is_place_holder<S: TreeRowOps>(store: &S, iter: &TreeIter) -> bool {
                store.get_value(iter, NAME).get_ok_some::<String>().as_str() == "(empty)"
            }

            fn get_name_from_row<S: TreeRowOps>(store: &S, iter: &TreeIter) -> String {
                store.get_value(iter, NAME).get_ok_some::<String>()
            }

            fn get_path_from_row<S: TreeRowOps>(store: &S, iter: &TreeIter) -> String {
                store.get_value(iter, PATH).get_ok_some::<String>()
            }

            fn update_row_if_required<S: TreeRowOps>(&self, store: &S, iter: &TreeIter) -> bool {
                assert_eq!(
                    self.name,
                    store.get_value(iter, NAME).get_ok_some::<String>()
                );
                let mut changed = false;
                if self.path != store.get_value(iter, PATH).get_ok_some::<String>() {
                    store.set_value(iter, PATH as u32, &self.path.to_value());
                    changed = true;
                }
                if self.is_dir != store.get_value(iter, IS_DIR).get_ok_some::<bool>() {
                    store.set_value(iter, IS_DIR as u32, &self.is_dir.to_value());
                    if self.is_dir {
                        store.set_value(iter, ICON as u32, &"stock_directory".to_value());
                    } else {
                        store.set_value(iter, ICON as u32, &"stock_file".to_value());
                    }
                    changed = true;
                }
                changed
            }

            fn set_row_values<S: TreeRowOps>(&self, store: &S, iter: &TreeIter) {
                store.set_value(iter, NAME as u32, &self.name.to_value());
                store.set_value(iter, PATH as u32, &self.path.to_value());
                if self.is_dir {
                    store.set_value(iter, ICON as u32, &"gtk-directory".to_value());
                } else {
                    store.set_value(iter, ICON as u32, &"gtk-file".to_value());
                }
                store.set_value(iter, IS_DIR as u32, &self.is_dir.to_value());
            }

            fn set_place_holder_values<S: TreeRowOps>(store: &S, iter: &TreeIter) {
                store.set_value(iter, NAME as u32, &"(empty)".to_value());
                store.set_value(iter, PATH as u32, &"".to_value());
                store.set_value(iter, IS_DIR as u32, &false.to_value());
            }

            fn name(&self) -> &str {
                &self.name
            }

            fn path(&self) -> &str {
                &self.path
            }

            fn is_dir(&self) -> bool {
                self.is_dir
            }
        }
    };
}

mod simple_os_fs_db {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::io::Write;

    use crypto_hash::{Algorithm, Hasher};

    use glib::{types::StaticType, value::ToValue};
    use gtk::prelude::*;
    use gtk::TreeIter;

    use super::*;

    use pw_pathux::str_path::*;

    impl_simple_fs_object!(SimpleFso);
    impl_os_fs_db!(OsFsDb, OsFsDbDir);
}
