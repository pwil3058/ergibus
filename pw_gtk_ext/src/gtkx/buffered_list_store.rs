// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>
use std::cell::RefCell;
use std::rc::Rc;

pub use crate::gtkx::list_store::*;
pub use crate::gtkx::value::Row;

pub trait RowDataSource: ListViewSpec + Sized {
    fn rows_and_digest(&self) -> (Vec<Row>, Vec<u8>);
    fn digest(&self) -> Vec<u8>;
}

#[derive(Default)]
pub struct Rows {
    rows: Rc<Vec<Row>>,
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

    fn set_rows_and_digest(&self) {
        let mut row_data = self.row_data.borrow_mut();
        let (rows, digest) = self.row_data_source.rows_and_digest();
        row_data.rows = Rc::new(rows);
        row_data.rows_digest = digest;
    }

    fn get_rows(&self) -> Rc<Vec<Row>> {
        let row_data = self.row_data.borrow();
        Rc::clone(&row_data.rows)
    }

    fn is_current(&self) -> bool {
        let row_data = self.row_data.borrow();
        row_data.rows_digest == self.row_data_source.digest()
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
        self.row_buffer.set_rows_and_digest();
        self.list_store.repopulate_with(&self.row_buffer.get_rows());
    }

    pub fn update(&self) {
        if !self.row_buffer.is_current() {
            // this does a raw data update
            self.row_buffer.set_rows_and_digest();
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
