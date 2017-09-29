// Copyright 2017 Peter Williams <pwil3058@gmail.com>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std;
use std::cell::RefCell;
use std::rc::Rc;

use gtk;
use gtk::prelude::*;

pub type Digest = Vec<u8>;

pub fn invalid_digest() -> Digest {
    Digest::default()
}

pub type Row = Vec<gtk::Value>;

#[derive(Default)]
pub struct RowBufferCore<RawData: Default> {
    pub raw_data: Rc<RawData>,
    pub raw_data_digest: Rc<Digest>,
    pub rows: Rc<Vec<Row>>,
    pub rows_digest:  Rc<Digest>,
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

    fn init(&self)  {
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

macro_rules! set_row_values {
    ( $s:expr, $i:expr, $r:expr ) => {
        {
            assert_eq!($s.get_n_columns(), $r.len() as i32);
            for (index, item) in $r.iter().enumerate() {
                $s.set_value(&$i, index as u32, &item);
            }
        }
    }
}

macro_rules! are_eq_values {
    ( $v1:expr, $v2:expr ) => {
        {
            let v1_type = $v1.type_();
            assert_eq!(v1_type, $v2.type_());
            // TODO: handle more types
            // TODO: panic if extracted values are None
            match v1_type {
                gtk::Type::String => {
                    let v1: Option<String> = $v1.get();
                    let v2: Option<String> = $v2.get();
                    v1 == v2
                },
                _ => panic!("operation not defined for: {:?}", v1_type)
            }
        }
    }
}

macro_rules! matches_list_row {
    ( $r:expr, $s:expr, $i:expr ) => {
        {
            assert_eq!($s.get_n_columns(), $r.len() as i32);
            let mut result = true;
            for (index, item) in $r.iter().enumerate() {
                let value = $s.get_value(&$i, index as i32);
                if !are_eq_values!(item, value) {
                    result = false;
                    break
                }
            };
            result
        }
    }
}

pub trait SimpleRowOps {
    fn get_list_store(&self) -> gtk::ListStore;

    fn append_row(&self, row: &Row) -> gtk::TreeIter {
        let list_store = self.get_list_store();
        let iter = list_store.append();
        set_row_values!(list_store, iter, row);
        iter
    }

    fn find_row_index(&self, row: &Row) -> Option<i32> {
        let list_store = self.get_list_store();
        let mut index: i32 = 0;
        if let Some(iter) = list_store.get_iter_first() {
            loop {
                if matches_list_row!(row, list_store, iter) {
                    return Some(index);
                };
                index += 1;
                if !list_store.iter_next(&iter) {
                    break;
                }
            }
        };
        None
    }

    fn find_row_iter(&self, row: &Row) -> Option<gtk::TreeIter> {
        let list_store = self.get_list_store();
        if let Some(iter) = list_store.get_iter_first() {
            loop {
                if matches_list_row!(row, list_store, iter) {
                    return Some(iter);
                };
                if !list_store.iter_next(&iter) {
                    break;
                }
            }
        };
        None
    }

    fn insert_row(&self, position: i32, row: &Row) -> gtk::TreeIter {
        let list_store = self.get_list_store();
        let iter = list_store.insert(position);
        set_row_values!(list_store, iter, row);
        iter
    }

    fn prepend_row(&self, row: &Row)  -> gtk::TreeIter {
        let list_store = self.get_list_store();
        let iter = list_store.prepend();
        set_row_values!(list_store, iter, row);
        iter
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn row_data_buffer_works() {
        struct TestBuffer {
            id: u8,
            row_buffer_core: Rc<RefCell<RowBufferCore<Vec<String>>>>
        }

        impl TestBuffer {
            pub fn new() -> TestBuffer {
                let mut row_buffer_core = RowBufferCore::<Vec<String>>::default();
                let buf = TestBuffer{id:0, row_buffer_core: Rc::new(RefCell::new(row_buffer_core))};
                buf.init();
                buf
            }

            pub fn set_id(&mut self, value: u8) {
                self.id = value;
            }
        }

        impl RowBuffer<Vec<String>> for TestBuffer {
            fn get_core(&self) -> Rc<RefCell<RowBufferCore<Vec<String>>>> {
                self.row_buffer_core.clone()
            }

            fn set_raw_data(&self) {
                let mut core = self.row_buffer_core.borrow_mut();
                match self.id {
                    0 => {
                        core.set_raw_data(Vec::new(), Vec::new());
                    },
                    1 => {
                        core.set_raw_data(
                            vec!["one".to_string(), "two".to_string(), "three".to_string()],
                            vec![1, 2, 3]
                        );
                    },
                    _ => {
                        core.set_raw_data(Vec::new(), Vec::new());
                    }
                }
            }

            fn finalise(&self){
                let mut core = self.row_buffer_core.borrow_mut();
                let mut rows: Vec<Row> = Vec::new();
                for item in core.raw_data.iter() {
                    rows.push(vec![item.to_value()]);
                };
                core.rows = Rc::new(rows);
                core.set_is_finalised_true();
            }
        }

        let mut buffer = TestBuffer::new();

        assert_eq!(buffer.get_rows().len(), 0);
        assert!(buffer.needs_init());
        assert!(!buffer.needs_finalise());
        assert!(buffer.is_current());

        buffer.set_id(1);
        assert!(!buffer.is_current());
        assert_eq!(buffer.get_rows().len(), 0);
        buffer.reset();
        assert!(buffer.is_current());
        let rows = buffer.get_rows();
        assert_eq!(rows[0][0].get(), Some("one"));
        assert_eq!(rows[1][0].get(), Some("two"));
        assert_eq!(rows[2][0].get(), Some("three"));
    }

    #[test]
    fn simple_row_ops_work()  {
        if !gtk::is_initialized() {
            gtk::init();
        }

        struct TestListStore {
            list_store: gtk::ListStore
        }

        impl TestListStore {
            pub fn new() -> TestListStore {
                let list_store = gtk::ListStore::new(&[gtk::Type::String, gtk::Type::String, gtk::Type::String]);
                TestListStore{list_store: list_store}
            }
        }

        impl SimpleRowOps for TestListStore {
            fn get_list_store(&self) -> gtk::ListStore {
                self.list_store.clone()
            }
        }

        let test_list_store = TestListStore::new();
        let row1 = vec!["one".to_value(), "two".to_value(), "three".to_value()];
        let row2 = vec!["four".to_value(), "five".to_value(), "six".to_value()];
        let row3 = vec!["seven".to_value(), "eight".to_value(), "nine".to_value()];
        test_list_store.append_row(&row1);
        assert_eq!(test_list_store.find_row_index(&row1), Some(0));
        assert_eq!(test_list_store.find_row_index(&row2), None);
        assert_eq!(test_list_store.find_row_index(&row3), None);
        test_list_store.prepend_row(&row2);
        assert_eq!(test_list_store.find_row_index(&row1), Some(1));
        assert_eq!(test_list_store.find_row_index(&row2), Some(0));
        assert_eq!(test_list_store.find_row_index(&row3), None);
        test_list_store.insert_row(1, &row3);
        assert_eq!(test_list_store.find_row_index(&row1), Some(2));
        assert_eq!(test_list_store.find_row_index(&row2), Some(0));
        assert_eq!(test_list_store.find_row_index(&row3), Some(1));
    }

}
