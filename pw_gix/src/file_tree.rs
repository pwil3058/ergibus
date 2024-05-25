// Copyright 2019 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::rc::Rc;

use gtk;
use gtk::prelude::*;

use crate::fs_db::{FsDbIfce, FsObjectIfce};
pub use crate::gtkx::tree_model::TreeModelRowOps;
pub use crate::gtkx::tree_store::TreeRowOps;
use crate::wrapper::WidgetWrapper;

trait FileTreeStoreExt: TreeRowOps {
    fn recursive_remove(&self, iter: &gtk::TreeIter) -> bool {
        if let Some(child_iter) = self.iter_children(Some(iter)) {
            while self.recursive_remove(&child_iter) {}
        }
        self.remove(iter)
    }

    fn remove_dead_rows<'a, F: Fn(&Self, &gtk::TreeIter) -> bool>(
        &self,
        mut o_iter: Option<&'a gtk::TreeIter>,
        until: F,
        changed: &mut bool,
    ) -> Option<&'a gtk::TreeIter> {
        while let Some(iter) = o_iter {
            if until(self, iter) {
                break;
            }
            o_iter = if self.recursive_remove(iter) {
                Some(iter)
            } else {
                None
            };
            *changed = true;
        }
        o_iter
    }
}

impl FileTreeStoreExt for gtk::TreeStore {}

pub trait FileTreeIfce<FSDB, FSOI>: WidgetWrapper
where
    FSDB: FsDbIfce<FSOI>,
    FSOI: FsObjectIfce,
{
    fn view(&self) -> &gtk::TreeView;
    fn store(&self) -> &gtk::TreeStore;
    fn fs_db(&self) -> &FSDB;
    fn auto_expand(&self) -> bool;
    fn show_hidden(&self) -> bool;
    fn hide_clean(&self) -> bool;

    fn get_fso_data_at(&self, posn: (f64, f64)) -> Option<(String, bool)> {
        let x = posn.0 as i32;
        let y = posn.1 as i32;
        if let Some(location) = self.view().get_path_at_pos(x, y) {
            if let Some(path) = location.0 {
                if let Some(iter) = self.store().get_iter(&path) {
                    let name = FSOI::get_path_from_row(self.store(), &iter);
                    let is_dir = FSOI::row_is_a_dir(self.store(), &iter);
                    return Some((name, is_dir));
                }
            }
        }
        None
    }

    fn insert_place_holder(&self, dir_iter: &gtk::TreeIter) {
        let iter = self.store().append(Some(dir_iter));
        FSOI::set_place_holder_values(self.store(), &iter);
    }

    fn insert_place_holder_if_needed(&self, dir_iter: &gtk::TreeIter) {
        if self.store().iter_n_children(Some(dir_iter)) == 0 {
            self.insert_place_holder(dir_iter)
        }
    }

    fn remove_place_holder(&self, dir_iter: &gtk::TreeIter) {
        if let Some(child_iter) = self.store().iter_children(Some(dir_iter)) {
            if FSOI::row_is_place_holder(self.store(), &child_iter) {
                self.store().remove(&child_iter);
            }
        }
    }

    fn not_yet_populated(&self, dir_iter: &gtk::TreeIter) -> bool {
        if self.store().iter_n_children(Some(dir_iter)) < 2 {
            if let Some(child_iter) = self.store().iter_children(Some(dir_iter)) {
                FSOI::row_is_place_holder(self.store(), &child_iter)
            } else {
                true
            }
        } else {
            false
        }
    }

    fn expand_row(&self, dir_iter: &gtk::TreeIter) {
        if self.not_yet_populated(dir_iter) {
            let path = FSOI::get_path_from_row(self.store(), dir_iter);
            self.populate_dir(&path, Some(dir_iter));
            if self.store().iter_n_children(Some(dir_iter)) > 1 {
                self.remove_place_holder(dir_iter)
            }
        }
    }

    fn depopulate(&self, iter: &gtk::TreeIter) {
        if let Some(ref child_iter) = self.store().iter_children(Some(iter)) {
            while self.store().recursive_remove(child_iter) {}
        }
        self.insert_place_holder(iter)
    }

    fn get_dir_contents(&self, dir_path: &str) -> (Rc<Vec<FSOI>>, Rc<Vec<FSOI>>) {
        self.fs_db()
            .dir_contents(dir_path, self.show_hidden(), self.hide_clean())
    }

    fn view_expand_row(&self, dir_iter: &gtk::TreeIter) {
        if let Some(ref path) = self.store().get_path(dir_iter) {
            self.view().expand_row(path, true);
        }
    }

    fn view_row_expanded(&self, iter: &gtk::TreeIter) -> bool {
        if let Some(ref path) = self.store().get_path(iter) {
            return self.view().row_expanded(path);
        }
        false
    }

    fn auto_expand_dir_or_insert_place_holder(&self, dir_path: &str, dir_iter: &gtk::TreeIter) {
        if self.auto_expand() {
            self.populate_dir(dir_path, Some(dir_iter));
            self.view_expand_row(dir_iter);
        } else {
            self.insert_place_holder(dir_iter);
        }
    }

    fn populate_dir(&self, dir_path: &str, o_parent_iter: Option<&gtk::TreeIter>) {
        let (dirs, files) = self.get_dir_contents(dir_path);
        for dir_data in dirs.iter() {
            let dir_iter = self.store().append(o_parent_iter);
            dir_data.set_row_values(self.store(), &dir_iter);
            self.auto_expand_dir_or_insert_place_holder(dir_data.path(), &dir_iter);
        }
        for file_data in files.iter() {
            let file_iter = self.store().append(o_parent_iter);
            file_data.set_row_values(self.store(), &file_iter);
        }
        if let Some(parent_iter) = o_parent_iter {
            self.insert_place_holder_if_needed(parent_iter)
        }
    }

    fn repopulate(&self) {
        let cursor = self.show_busy();
        self.fs_db().reset();
        self.store().clear();
        if let Some(iter) = self.store().get_iter_first() {
            self.populate_dir(".", Some(&iter))
        } else {
            self.populate_dir(".", None)
        }
        self.unshow_busy(cursor);
    }

    fn update_dir(&self, dir_path: &str, o_parent_iter: Option<&gtk::TreeIter>) -> bool {
        // TODO: make sure we cater for case where dir becomes file and vice versa in a single update
        let mut changed = false;
        let mut o_place_holder_iter: Option<gtk::TreeIter> = None;
        let child_iter: gtk::TreeIter; // needed to satisfy lifetimes
        let mut o_child_iter: Option<&gtk::TreeIter> = if let Some(parent_iter) = o_parent_iter {
            if let Some(iter) = self.store().iter_children(Some(parent_iter)) {
                child_iter = iter;
                if FSOI::row_is_place_holder(self.store(), &child_iter) {
                    o_place_holder_iter = Some(child_iter.clone());
                    self.store().get_iter_next(&child_iter)
                } else {
                    Some(&child_iter)
                }
            } else {
                None
            }
        } else if let Some(iter) = self.store().get_iter_first() {
            child_iter = iter;
            Some(&child_iter)
        } else {
            None
        };
        let (dirs, files) = self.get_dir_contents(dir_path);
        for dir_data in dirs.iter() {
            o_child_iter = self.store().remove_dead_rows(
                o_child_iter,
                |s, i| {
                    !FSOI::row_is_a_dir(s, i)
                        || FSOI::get_name_from_row(s, i).as_str() >= dir_data.name()
                },
                &mut changed,
            );
            if let Some(child_iter) = o_child_iter {
                let name = FSOI::get_name_from_row(self.store(), child_iter);
                if !FSOI::row_is_a_dir(self.store(), child_iter) || name.as_str() > dir_data.name()
                {
                    let dir_iter = self.store().insert_before(o_parent_iter, o_child_iter);
                    dir_data.set_row_values(self.store(), &dir_iter);
                    changed = true;
                    self.auto_expand_dir_or_insert_place_holder(dir_data.path(), &dir_iter);
                } else {
                    changed |= dir_data.update_row_if_required(self.store(), child_iter);
                    // This is an update so ignore auto_expand for existing directories
                    // BUT update them if they"re already expanded
                    if self.view_row_expanded(child_iter) {
                        changed |= self.update_dir(dir_data.path(), o_child_iter);
                    } else {
                        // make sure we don"t leave bad data in children that were previously expanded
                        self.depopulate(child_iter);
                    }
                    o_child_iter = self.store().get_iter_next(child_iter);
                }
            } else {
                let dir_iter = self.store().append(o_parent_iter);
                dir_data.set_row_values(self.store(), &dir_iter);
                changed = true;
                self.auto_expand_dir_or_insert_place_holder(dir_data.path(), &dir_iter);
            }
        }
        o_child_iter = self.store().remove_dead_rows(
            o_child_iter,
            |s, i| !FSOI::row_is_a_dir(s, i),
            &mut changed,
        );
        for file_data in files.iter() {
            o_child_iter = self.store().remove_dead_rows(
                o_child_iter,
                |s, i| FSOI::get_name_from_row(s, i).as_str() >= file_data.name(),
                &mut changed,
            );
            if let Some(child_iter) = o_child_iter {
                if FSOI::get_name_from_row(self.store(), child_iter).as_str() > file_data.name() {
                    changed = true;
                    let file_iter = self.store().insert_before(o_parent_iter, o_child_iter);
                    file_data.set_row_values(self.store(), &file_iter);
                } else {
                    changed |= file_data.update_row_if_required(self.store(), child_iter);
                    o_child_iter = self.store().get_iter_next(child_iter);
                }
            } else {
                let file_iter = self.store().append(o_parent_iter);
                file_data.set_row_values(self.store(), &file_iter);
                changed = true;
            }
        }
        self.store()
            .remove_dead_rows(o_child_iter, |_, _| false, &mut changed);

        if let Some(parent_iter) = o_parent_iter {
            let n_children = self.store().iter_n_children(Some(parent_iter));
            if n_children == 0 {
                self.insert_place_holder(parent_iter)
            } else if n_children > 1 {
                if let Some(place_holder_iter) = o_place_holder_iter {
                    //assert_eq!(self.get_value(iter, 0), None);
                    self.store().remove(&place_holder_iter);
                }
            }
        }
        changed
    }

    fn update(&self) {
        let cursor = self.show_busy();
        if self.fs_db().update_if_necessary() {
            self.update_dir(".", None);
        }
        self.unshow_busy(cursor);
    }
}
