// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::glib::Value;
pub use crate::gtkx::list_store::*;
use std::cell::RefCell;
use std::rc::Rc;

pub trait RowDataSource: ListViewSpec + Sized {
    fn generate_rows(&self) -> Vec<Vec<Value>>;
    fn refresh(&self) -> Vec<u8>;
}

#[derive(Default)]
pub struct Rows {
    row_data_source_digest: Vec<u8>,
    rows: Rc<Vec<Vec<Value>>>,
    rows_digest: Vec<u8>,
}

pub struct RowBuffer<R: RowDataSource> {
    row_data_source: R,
    row_data: RefCell<Rows>,
}

impl<R: RowDataSource> RowBuffer<R> {
    pub fn new(raw_data: R) -> Self {
        RowBuffer {
            row_data_source: raw_data,
            row_data: RefCell::new(Rows::default()),
        }
    }

    pub fn columns() -> Vec<gtk::TreeViewColumn> {
        R::columns()
    }

    fn finalise(&self) {
        let mut row_data = self.row_data.borrow_mut();
        row_data.rows = Rc::new(self.row_data_source.generate_rows());
        row_data.rows_digest = row_data.row_data_source_digest.clone();
    }

    fn get_rows(&self) -> Rc<Vec<Vec<glib::Value>>> {
        let row_data = self.row_data.borrow();
        Rc::clone(&row_data.rows)
    }

    fn init(&self) {
        {
            let mut row_data = self.row_data.borrow_mut();
            row_data.row_data_source_digest = self.row_data_source.refresh();
        }
        self.finalise();
    }

    fn is_current(&self) -> bool {
        let mut row_data = self.row_data.borrow_mut();
        row_data.row_data_source_digest = self.row_data_source.refresh();
        row_data.row_data_source_digest == row_data.rows_digest
    }
}

pub struct BufferedListStore<R: RowDataSource> {
    list_store: gtk::ListStore,
    row_buffer: RowBuffer<R>,
}

impl<R: RowDataSource> BufferedListStore<R> {
    pub fn new(raw_data_source: R) -> Self {
        let list_store = gtk::ListStore::new(&R::column_types());
        let row_buffer = RowBuffer::new(raw_data_source);
        Self {
            list_store,
            row_buffer,
        }
    }

    pub fn row_data_source(&self) -> &R {
        &self.row_buffer.row_data_source
    }

    pub fn repopulate(&self) {
        self.row_buffer.init();
        self.list_store.repopulate_with(&self.row_buffer.get_rows());
    }

    pub fn update(&self) {
        if !self.row_buffer.is_current() {
            // this does a raw data update
            self.row_buffer.finalise();
            self.list_store.update_with(&self.row_buffer.get_rows());
        };
    }
}

impl<R: RowDataSource> WrappedTreeModel<gtk::ListStore> for BufferedListStore<R> {
    fn columns() -> Vec<gtk::TreeViewColumn> {
        R::columns()
    }

    fn model(&self) -> &gtk::ListStore {
        &self.list_store
    }
}
