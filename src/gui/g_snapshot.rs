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

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use gtk;
use gtk::prelude::*;

use crypto_hash::{Hasher, Algorithm};

use pw_gix::gtkx::list_store::{
    Row, RowBuffer, RowBufferCore, Digest, invalid_digest,
    SimpleRowOps
};
use snapshot;

struct SnapshotRowBuffer {
    archive_name: Option<String>,
    row_buffer_core: Rc<RefCell<RowBufferCore<Vec<String>>>>
}

fn generate_digest(list: &Vec<String>) -> Digest {
    let mut hasher = Hasher::new(Algorithm::SHA256);
    for ref item in list {
        if let Err(err) = hasher.write_all(item.as_bytes()){
            panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
        };
    };
    hasher.finish()
}

impl RowBuffer<Vec<String>> for SnapshotRowBuffer {
    fn get_core(&self) -> Rc<RefCell<RowBufferCore<Vec<String>>>> {
        self.row_buffer_core.clone()
    }

    fn set_raw_data(&self) {
        let mut core = self.row_buffer_core.borrow_mut();
        match self.archive_name {
            Some(ref archive_name) => {
                match snapshot::get_snapshot_names_for_archive(archive_name, true) {
                    Ok(mut snapshot_names) => {
                        let hash = generate_digest(&snapshot_names);
                        core.set_raw_data(snapshot_names, hash);
                    },
                    Err(_) => core.set_raw_data(Vec::new(), invalid_digest())
                }
            },
            None => core.set_raw_data(Vec::new(), invalid_digest())
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

impl SnapshotRowBuffer {
    fn new(archive_name: Option<String>) -> SnapshotRowBuffer {
        let core = RowBufferCore::<Vec<String>>::default();
        let mut buffer = SnapshotRowBuffer {
            archive_name: archive_name,
            row_buffer_core: Rc::new(RefCell::new(core)),
        };
        buffer.init();
        buffer
    }
}

struct SnapshotNameListStore {
    list_store: gtk::ListStore,
    snapshot_row_buffer: SnapshotRowBuffer
}

impl SimpleRowOps for SnapshotNameListStore {
    fn get_list_store(&self) -> gtk::ListStore {
        self.list_store.clone()
    }
}

impl SnapshotNameListStore {
    pub fn new(archive_name: Option<String>) -> SnapshotNameListStore {
        let mut list_store = SnapshotNameListStore {
            list_store: gtk::ListStore::new(&[gtk::Type::String]),
            snapshot_row_buffer: SnapshotRowBuffer::new(None)
        };
        list_store.set_archive_name(archive_name);
        list_store
    }

    pub fn set_archive_name(&mut self, archive_name: Option<String>) {
        if self.snapshot_row_buffer.archive_name == archive_name {
            return; // nothing to do
        }
        self.snapshot_row_buffer.archive_name = archive_name;
        self.populate();
    }

    pub fn populate(&mut self) {
        self.list_store.clear();
        self.snapshot_row_buffer.init();
        for row in self.snapshot_row_buffer.get_rows().iter() {
            self.append_row(row);
        }
    }

    pub fn update_contents(&mut self) {
        self.snapshot_row_buffer.reset();
        self.update_with(&*self.snapshot_row_buffer.get_rows());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn g_snapshot_list_store() {
        if !gtk::is_initialized() {
            if let Err(err) = gtk::init() {
                panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
            };
        }
        let mut store = SnapshotNameListStore::new(None);
        store.set_archive_name(Some("whatever".to_string()));
    }
}
