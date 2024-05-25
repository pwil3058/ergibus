//Copyright 2019 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use gtk;

pub use super::tree_model::{self, TreeModelRowOps};
pub use super::value::Row;

#[macro_export]
macro_rules! set_tree_row_values {
    ( $s:expr, $i:expr, $r:expr ) => {{
        assert_eq!($s.get_n_columns(), $r.len() as i32);
        for (index, item) in $r.iter().enumerate() {
            $s.set_value($i, index as u32, &item);
        }
    }};
}

#[macro_export]
macro_rules! append_row_to_tree {
    ( $r:expr, $s:expr, $i:expr ) => {{
        let iter = $s.append($i);
        set_tree_row_values!($s, &iter, $r);
        iter
    }};
}

#[macro_export]
macro_rules! insert_row_in_tree_at {
    ( $r:expr, $s:expr, $p:expr, $i:expr ) => {{
        let iter = $s.insert($p, $i);
        set_list_row_values!($s, &iter, $r);
        iter
    }};
}

#[macro_export]
macro_rules! insert_row_in_tree_after {
    ( $r:expr, $s:expr, $p:expr, $q:expr ) => {{
        let iter = $s.insert_after($p, $q);
        set_list_row_values!($s, &iter, $r);
        iter
    }};
}

#[macro_export]
macro_rules! insert_row_in_tree_before {
    ( $r:expr, $s:expr, $p:expr, $q:expr ) => {{
        let iter = $s.insert_before($p, $q);
        set_list_row_values!($s, &iter, $r);
        iter
    }};
}

#[macro_export]
macro_rules! prepend_row_to_tree {
    ( $r:expr, $s:expr, $i:expr ) => {{
        let iter = $s.prepend($i);
        set_tree_row_values!($s, &iter, $r);
        iter
    }};
}

pub trait TreeRowOps:
    TreeModelRowOps + gtk::TreeStoreExt + gtk::prelude::TreeStoreExtManual
{
    fn append_row(&self, row: &[glib::Value], parent: Option<&gtk::TreeIter>) -> gtk::TreeIter {
        append_row_to_tree!(row, self, parent)
    }

    fn insert_row(
        &self,
        row: &[glib::Value],
        parent: Option<&gtk::TreeIter>,
        position: i32,
    ) -> gtk::TreeIter {
        insert_row_in_tree_at!(row, self, parent, position)
    }

    fn insert_row_after(
        &self,
        row: &[glib::Value],
        parent: Option<&gtk::TreeIter>,
        sibling: Option<&gtk::TreeIter>,
    ) -> gtk::TreeIter {
        insert_row_in_tree_after!(row, self, parent, sibling)
    }

    fn insert_row_before(
        &self,
        row: &[glib::Value],
        parent: Option<&gtk::TreeIter>,
        sibling: Option<&gtk::TreeIter>,
    ) -> gtk::TreeIter {
        insert_row_in_tree_before!(row, self, parent, sibling)
    }

    fn prepend_row(&self, row: &[glib::Value], parent: Option<&gtk::TreeIter>) -> gtk::TreeIter {
        prepend_row_to_tree!(row, self, parent)
    }

    fn set_row_values(&self, row: &[glib::Value], iter: &gtk::TreeIter) {
        set_tree_row_values!(self, iter, row);
    }
}

impl TreeRowOps for gtk::TreeStore {}
