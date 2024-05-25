// Copyright 2017 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use gtk;
use gtk::prelude::*;

pub use super::value::Row;
pub use crate::glibx::*;

// NB: when done with the returned row it's items need to be unset?
#[macro_export]
macro_rules! get_row_values_from {
    ( $s:expr, $i:expr ) => {{
        let mut row = Row::new();
        let n = $s.get_n_columns();
        for index in 0..n {
            row.push($s.get_value($i, index))
        }
        row
    }};
}

// NB: when done with the returned row it's items need to be unset?
#[macro_export]
macro_rules! get_row_values_from_at {
    ( $s:expr, $p:expr ) => {{
        match $s.iter_nth_child(None, $p) {
            Some(iter) => Some(get_row_values_from!($s, &iter)),
            None => None,
        }
    }};
}

#[macro_export]
macro_rules! matches_list_row {
    ( $r:expr, $s:expr, $i:expr ) => {{
        assert_eq!($s.get_n_columns(), $r.len() as i32);
        let mut result = true;
        for (index, item) in $r.iter().enumerate() {
            let value = $s.get_value($i, index as i32);
            if !are_eq_values!(item, value) {
                result = false;
                break;
            }
        }
        result
    }};
}

#[macro_export]
macro_rules! len_of {
    ( $s:expr ) => {{
        $s.iter_n_children(None)
    }};
}

pub trait TreeModelRowOps: TreeModelExt {
    fn len(&self) -> i32 {
        len_of!(self)
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn find_row_where<F>(&self, this_is_the_row: F) -> Option<(i32, gtk::TreeIter)>
    where
        F: Fn(&Self, &gtk::TreeIter) -> bool,
    {
        let mut index: i32 = 0;
        if let Some(iter) = self.get_iter_first() {
            loop {
                if this_is_the_row(self, &iter) {
                    return Some((index, iter));
                };
                index += 1;
                if !self.iter_next(&iter) {
                    break;
                }
            }
        };
        None
    }

    fn get_row_values(&self, iter: &gtk::TreeIter) -> Row {
        get_row_values_from!(self, iter)
    }

    fn get_row_values_at(&self, position: i32) -> Option<Row> {
        get_row_values_from_at!(self, position)
    }

    fn find_row(&self, row: &[glib::Value]) -> Option<(i32, gtk::TreeIter)> {
        self.find_row_where(|list_store, iter| matches_list_row!(row, list_store, iter))
    }

    fn find_row_index(&self, row: &[glib::Value]) -> Option<i32> {
        self.find_row(row).map(|(index, _)| index)
    }

    fn find_row_iter(&self, row: &[glib::Value]) -> Option<gtk::TreeIter> {
        self.find_row(row).map(|(_, iter)| iter)
    }

    fn get_iter_next<'a>(&self, iter: &'a gtk::TreeIter) -> Option<&'a gtk::TreeIter> {
        if self.iter_next(iter) {
            Some(iter)
        } else {
            None
        }
    }
}

impl TreeModelRowOps for gtk::ListStore {}
impl TreeModelRowOps for gtk::TreeStore {}
