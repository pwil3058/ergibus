// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::glib;
pub use crate::gtkx::tree_model::*;
use crate::{are_eq_values, are_equal_as, get_row_values_from, matches_list_row, UNEXPECTED};
use std::marker::PhantomData;
use std::ops::Deref;

// NB: when done with the returned rows their items need to be unset?
#[macro_export]
macro_rules! get_rows_values_from_list {
    ( $list_store:expr ) => {{
        let mut rows = vec![];
        if let Some(iter) = $list_store.get_iter_first() {
            while $list_store.iter_is_valid(&iter) {
                rows.push(get_row_values_from!($list_store, &iter));
                $list_store.iter_next(&iter);
            }
        };
        rows
    }};
}

#[macro_export]
macro_rules! set_list_row_values {
    ( $list_store:expr, $iter:expr, $row:expr ) => {{
        debug_assert_eq!($list_store.get_n_columns(), $row.len() as i32);
        for (index, item) in $row.iter().enumerate() {
            $list_store.set_value($iter, index as u32, &item);
        }
    }};
}

#[macro_export]
macro_rules! append_row_to_list {
    ( $row:expr, $list_store:expr ) => {{
        let iter = $list_store.append();
        set_list_row_values!($list_store, &iter, $row);
        iter
    }};
}

#[macro_export]
macro_rules! insert_row_in_list_at {
    ( $row:expr, $list_store:expr, $position:expr ) => {{
        let iter = $list_store.insert($position);
        set_list_row_values!($list_store, &iter, $row);
        iter
    }};
}

#[macro_export]
macro_rules! insert_row_in_list_after {
    ( $row:expr, $list_store:expr, $iter:expr ) => {{
        let iter = $list_store.insert_after($iter);
        set_list_row_values!($list_store, &iter, $row);
        iter
    }};
}

#[macro_export]
macro_rules! insert_row_in_list_before {
    ( $row:expr, $list_store:expr, $iter:expr ) => {{
        let iter = $list_store.insert_before($iter);
        set_list_row_values!($list_store, &iter, $row);
        iter
    }};
}

#[macro_export]
macro_rules! prepend_row_to_list {
    ( $row:expr, $list_store:expr ) => {{
        let iter = $list_store.prepend();
        set_list_row_values!($list_store, &iter, $row);
        iter
    }};
}

pub trait ListRowOps:
    TreeModelRowOps + gtk::GtkListStoreExt + gtk::prelude::GtkListStoreExtManual
{
    fn append_row(&self, row: &[glib::Value]) -> gtk::TreeIter {
        append_row_to_list!(row, self)
    }

    fn get_rows_values(&self) -> Vec<Vec<glib::Value>> {
        get_rows_values_from_list!(self)
    }

    fn insert_row(&self, position: i32, row: &[glib::Value]) -> gtk::TreeIter {
        insert_row_in_list_at!(row, self, position)
    }

    fn insert_row_after(&self, iter: &gtk::TreeIter, row: &[glib::Value]) -> gtk::TreeIter {
        insert_row_in_list_after!(row, self, Some(iter))
    }

    fn insert_row_before(&self, iter: &gtk::TreeIter, row: &[glib::Value]) -> gtk::TreeIter {
        insert_row_in_list_before!(row, self, Some(iter))
    }

    fn prepend_row(&self, row: &[glib::Value]) -> gtk::TreeIter {
        prepend_row_to_list!(row, self)
    }

    fn remove_row_at(&self, position: i32) -> Result<(), &'static str> {
        match self.iter_nth_child(None, position) {
            Some(iter) => {
                self.remove(&iter);
                Ok(())
            }
            None => Err("invalid position"),
        }
    }

    fn remove_row_where<F>(&self, this_is_the_row: F) -> Result<(), &'static str>
    where
        F: Fn(&Self, &gtk::TreeIter) -> bool,
    {
        if let Some((_, iter)) = self.find_row_where(this_is_the_row) {
            self.remove(&iter);
            Ok(())
        } else {
            Err("not found")
        }
    }

    fn repopulate_with(&self, rows: &[Vec<glib::Value>]) {
        self.clear();
        for row in rows.iter() {
            append_row_to_list!(row, self);
        }
    }

    // NB: this function assumes that all rows are unique and that order isn't important
    fn update_with(&self, rows: &[Vec<glib::Value>]) {
        // First remove the rows that have gone away
        if let Some(iter) = self.get_iter_first() {
            while self.iter_is_valid(&iter) {
                let mut found = false;
                for row in rows.iter() {
                    if matches_list_row!(row, self, &iter) {
                        found = true;
                        break;
                    }
                }
                if !found {
                    // NB: this call updates the iter
                    self.remove(&iter);
                } else {
                    self.iter_next(&iter);
                }
            }
        }
        // Now add any new ones
        for row in rows.iter() {
            let position = self.find_row(row);
            if position.is_none() {
                self.append_row(row);
            }
        }
    }
}

impl ListRowOps for gtk::ListStore {}

pub trait ListViewSpec {
    fn column_types() -> Vec<glib::Type>;
    fn columns() -> Vec<gtk::TreeViewColumn>;
}

#[derive(Clone)]
pub struct WrappedListStore<L: ListViewSpec>(gtk::ListStore, PhantomData<L>);

impl<L: ListViewSpec> Deref for WrappedListStore<L> {
    type Target = gtk::ListStore;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<L: ListViewSpec> WrappedTreeModel<gtk::ListStore> for WrappedListStore<L> {
    fn columns() -> Vec<gtk::TreeViewColumn> {
        L::columns()
    }

    fn model(&self) -> &gtk::ListStore {
        &self.0
    }
}

impl<L: ListViewSpec> WrappedListStore<L> {
    pub fn new() -> Self {
        Self(gtk::ListStore::new(&L::column_types()), PhantomData)
    }
}

impl<L: ListViewSpec> Default for WrappedListStore<L> {
    fn default() -> Self {
        Self(gtk::ListStore::new(&L::column_types()), PhantomData)
    }
}
