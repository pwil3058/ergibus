// Copyright 2017 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::cell::{Ref, RefCell};
use std::rc::Rc;

use gtk;

pub use super::tree_model::{self, TreeModelRowOps};
pub use super::value::Row;
pub use crate::glibx::*;

// NB: when done with the returned rows their items need to be unset?
#[macro_export]
macro_rules! get_rows_values_from_list {
    ( $s:expr ) => {{
        let mut rows: Vec<Row> = Vec::new();
        if let Some(iter) = $s.get_iter_first() {
            while $s.iter_is_valid(&iter) {
                rows.push(get_row_values_from!($s, &iter));
                $s.iter_next(&iter);
            }
        };
        rows
    }};
}

#[macro_export]
macro_rules! set_list_row_values {
    ( $s:expr, $i:expr, $r:expr ) => {{
        assert_eq!($s.get_n_columns(), $r.len() as i32);
        for (index, item) in $r.iter().enumerate() {
            $s.set_value($i, index as u32, &item);
        }
    }};
}

#[macro_export]
macro_rules! append_row_to_list {
    ( $r:expr, $s:expr ) => {{
        let iter = $s.append();
        set_list_row_values!($s, &iter, $r);
        iter
    }};
}

#[macro_export]
macro_rules! insert_row_in_list_at {
    ( $r:expr, $s:expr, $p:expr ) => {{
        let iter = $s.insert($p);
        set_list_row_values!($s, &iter, $r);
        iter
    }};
}

#[macro_export]
macro_rules! insert_row_in_list_after {
    ( $r:expr, $s:expr, $i:expr ) => {{
        let iter = $s.insert_after($i);
        set_list_row_values!($s, &iter, $r);
        iter
    }};
}

#[macro_export]
macro_rules! insert_row_in_list_before {
    ( $r:expr, $s:expr, $i:expr ) => {{
        let iter = $s.insert_before($i);
        set_list_row_values!($s, &iter, $r);
        iter
    }};
}

#[macro_export]
macro_rules! prepend_row_to_list {
    ( $r:expr, $s:expr ) => {{
        let iter = $s.prepend();
        set_list_row_values!($s, &iter, $r);
        iter
    }};
}

pub trait ListRowOps:
    TreeModelRowOps + gtk::GtkListStoreExt + gtk::prelude::GtkListStoreExtManual
{
    fn append_row(&self, row: &[glib::Value]) -> gtk::TreeIter {
        append_row_to_list!(row, self)
    }

    fn get_rows_values(&self) -> Vec<Row> {
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

    fn repopulate_with(&self, rows: &[Row]) {
        self.clear();
        for row in rows.iter() {
            append_row_to_list!(row, self);
        }
    }

    // NB: this function assumes that all rows are unique and that order isn't important
    fn update_with(&self, rows: &[Row]) {
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

pub type Digest = Vec<u8>;

pub fn invalid_digest() -> Digest {
    Digest::default()
}

#[derive(Default)]
pub struct RowBufferCore<RawData: Default> {
    pub raw_data: Rc<RawData>,
    pub raw_data_digest: Rc<Digest>,
    pub rows: Rc<Vec<Row>>,
    pub rows_digest: Rc<Digest>,
}

impl<RawData: Default> RowBufferCore<RawData> {
    pub fn is_finalised(&self) -> bool {
        self.rows_digest == self.raw_data_digest
    }

    pub fn needs_init(&self) -> bool {
        self.raw_data_digest == Rc::new(invalid_digest())
    }

    pub fn set_raw_data(&mut self, raw_data: RawData, raw_data_digest: Digest) {
        self.raw_data = Rc::new(raw_data);
        self.raw_data_digest = Rc::new(raw_data_digest);
    }

    pub fn set_is_finalised_true(&mut self) {
        self.rows_digest = self.raw_data_digest.clone();
    }
}

pub trait RowBuffer<RawData: Default> {
    fn get_core(&self) -> Rc<RefCell<RowBufferCore<RawData>>>;
    fn set_raw_data(&self);
    fn finalise(&self);

    fn needs_finalise(&self) -> bool {
        let core = self.get_core();
        let answer = core.borrow().is_finalised();
        !answer
    }

    fn needs_init(&self) -> bool {
        let core = self.get_core();
        let answer = core.borrow().needs_init();
        answer
    }

    fn init(&self) {
        self.set_raw_data();
        self.finalise();
    }

    fn is_current(&self) -> bool {
        self.set_raw_data();
        !self.needs_finalise()
    }

    fn reset(&self) {
        if self.needs_init() {
            self.init();
        } else if self.needs_finalise() {
            self.finalise();
        }
    }

    fn get_rows(&self) -> Rc<Vec<Row>> {
        let core = self.get_core();
        let rows = core.borrow().rows.clone();
        rows
    }
}

pub trait BufferedUpdate<RawData: Default, L: ListRowOps> {
    fn get_list_store(&self) -> L;
    fn get_row_buffer(&self) -> Rc<RefCell<dyn RowBuffer<RawData>>>;

    fn repopulate(&self) {
        let row_buffer_rc = self.get_row_buffer();
        let row_buffer = row_buffer_rc.borrow();

        row_buffer.init();
        self.get_list_store()
            .repopulate_with(&row_buffer.get_rows());
    }

    fn update(&self) {
        let row_buffer_rc = self.get_row_buffer();
        let row_buffer = row_buffer_rc.borrow();

        if !row_buffer.is_current() {
            // this does a raw data update
            row_buffer.finalise();
            self.get_list_store().update_with(&row_buffer.get_rows());
        };
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RequiredMapAction {
    Repopulate,
    Update,
    Nothing,
}

pub trait MapManagedUpdate<Buffer, RawData, List>
where
    Buffer: BufferedUpdate<RawData, List>,
    RawData: Default,
    List: ListRowOps,
{
    fn buffered_update(&self) -> Ref<'_, Buffer>;
    fn is_mapped(&self) -> bool;
    fn get_required_map_action(&self) -> RequiredMapAction;
    fn set_required_map_action(&self, action: RequiredMapAction);

    fn auto_update(&self) {
        if self.get_required_map_action() == RequiredMapAction::Nothing {
            self.update()
        }
    }

    fn on_map_action(&self) {
        match self.get_required_map_action() {
            RequiredMapAction::Repopulate => {
                self.repopulate();
                self.set_required_map_action(RequiredMapAction::Nothing);
            }
            RequiredMapAction::Update => {
                self.update();
                self.set_required_map_action(RequiredMapAction::Nothing);
            }
            RequiredMapAction::Nothing => (),
        }
    }

    fn repopulate(&self) {
        if self.is_mapped() {
            self.buffered_update().repopulate();
            self.set_required_map_action(RequiredMapAction::Nothing)
        } else {
            self.set_required_map_action(RequiredMapAction::Repopulate)
        }
    }

    fn update(&self) {
        if self.is_mapped() {
            self.buffered_update().update();
            self.set_required_map_action(RequiredMapAction::Nothing)
        } else if self.get_required_map_action() != RequiredMapAction::Repopulate {
            self.set_required_map_action(RequiredMapAction::Update)
        }
    }
}
