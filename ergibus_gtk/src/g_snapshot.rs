use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use pw_gix::{
    glib,
    gtk::{self, prelude::*},
    gtkx::list_store::*,
    wrapper::*,
};

use crypto_hash::{Algorithm, Hasher};

use ergibus_lib::snapshot_ng;

use crate::g_archive;

struct SnapshotRowBuffer {
    archive_name: Option<String>,
    row_buffer_core: Rc<RefCell<RowBufferCore<Vec<String>>>>,
}

fn generate_digest(list: &Vec<String>) -> Digest {
    let mut hasher = Hasher::new(Algorithm::SHA256);
    for ref item in list {
        if let Err(err) = hasher.write_all(item.as_bytes()) {
            panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
        };
    }
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
                match snapshot_ng::get_snapshot_names_for_archive(archive_name, true) {
                    Ok(snapshot_names) => {
                        let hash = generate_digest(&snapshot_names);
                        core.set_raw_data(snapshot_names, hash);
                    }
                    Err(_) => core.set_raw_data(Vec::new(), invalid_digest()),
                }
            }
            None => core.set_raw_data(Vec::new(), invalid_digest()),
        }
    }

    fn finalise(&self) {
        let mut core = self.row_buffer_core.borrow_mut();
        let mut rows: Vec<Row> = Vec::new();
        for item in core.raw_data.iter() {
            rows.push(vec![item.to_value()]);
        }
        core.rows = Rc::new(rows);
        core.set_is_finalised_true();
    }
}

impl SnapshotRowBuffer {
    fn new(archive_name: Option<String>) -> SnapshotRowBuffer {
        let core = RowBufferCore::<Vec<String>>::default();
        let buffer = SnapshotRowBuffer {
            archive_name: archive_name,
            row_buffer_core: Rc::new(RefCell::new(core)),
        };
        buffer.init();
        buffer
    }
}

struct SnapshotNameListStore {
    list_store: gtk::ListStore,
    snapshot_row_buffer: Rc<RefCell<SnapshotRowBuffer>>,
}

impl BufferedUpdate<Vec<String>, gtk::ListStore> for SnapshotNameListStore {
    fn get_list_store(&self) -> gtk::ListStore {
        self.list_store.clone()
    }

    fn get_row_buffer(&self) -> Rc<RefCell<dyn RowBuffer<Vec<String>>>> {
        self.snapshot_row_buffer.clone()
    }
}

impl SnapshotNameListStore {
    pub fn new(archive_name: Option<String>) -> SnapshotNameListStore {
        let mut list_store = SnapshotNameListStore {
            list_store: gtk::ListStore::new(&[glib::Type::String]),
            snapshot_row_buffer: Rc::new(RefCell::new(SnapshotRowBuffer::new(None))),
        };
        list_store.set_archive_name(archive_name);
        list_store
    }

    pub fn set_archive_name(&mut self, archive_name: Option<String>) {
        println!("set archive: {:?}", archive_name);
        if self.snapshot_row_buffer.borrow().archive_name == archive_name {
            return; // nothing to do
        }
        self.snapshot_row_buffer.borrow_mut().archive_name = archive_name;
        self.repopulate();
    }
}

#[derive(PWO, Wrapper)]
pub struct SnapshotNameTable {
    pub view: gtk::TreeView,
    list_store: RefCell<SnapshotNameListStore>,
}

impl SnapshotNameTable {
    pub fn new_rc(archive_name: Option<String>) -> Rc<SnapshotNameTable> {
        let list_store = RefCell::new(SnapshotNameListStore::new(archive_name));

        let view = gtk::TreeView::with_model(&list_store.borrow().get_list_store());
        view.set_headers_visible(true);

        view.get_selection().set_mode(gtk::SelectionMode::Multiple);

        let col = gtk::TreeViewColumn::new();
        col.set_title("Snapshot Time"); // I18N need here
        col.set_expand(false);
        col.set_resizable(false);

        let cell = gtk::CellRendererText::new();
        cell.set_property_editable(false);
        cell.set_property_max_width_chars(29);
        cell.set_property_width_chars(29);
        cell.set_property_xalign(0.0);

        col.pack_start(&cell, false);
        col.add_attribute(&cell, "text", 0);

        view.append_column(&col);
        view.show_all();

        Rc::new(SnapshotNameTable { view, list_store })
    }

    pub fn set_archive(&self, archive_name: Option<String>) {
        self.list_store.borrow_mut().set_archive_name(archive_name);
    }
}

#[derive(PWO, Wrapper)]
pub struct SnapshotSelector {
    vbox: gtk::Box,
    archive_selector: Rc<g_archive::ArchiveSelector>,
    snapshot_name_table: Rc<SnapshotNameTable>,
}

impl SnapshotSelector {
    pub fn new_rc() -> Rc<SnapshotSelector> {
        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let archive_selector = g_archive::ArchiveSelector::new_rc();
        vbox.pack_start(&archive_selector.pwo(), false, false, 0);
        let snapshot_name_table =
            SnapshotNameTable::new_rc(archive_selector.get_selected_archive());
        vbox.pack_start(&snapshot_name_table.pwo(), false, false, 0);
        vbox.show_all();
        let snapshot_selector = Rc::new(SnapshotSelector {
            vbox,
            archive_selector,
            snapshot_name_table,
        });

        let snt = snapshot_selector.snapshot_name_table.clone();
        snapshot_selector
            .archive_selector
            .connect_changed(move |new_archive_name| snt.set_archive(new_archive_name));

        snapshot_selector
    }
}
