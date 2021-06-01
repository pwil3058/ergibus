use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use pw_gtk_ext::{
    glib,
    gtk::{self, prelude::*},
    sav_state::SAV_SELN_UNIQUE_OR_HOVER_OK,
    wrapper::*,
    UNEXPECTED,
};

use crypto_hash::{Algorithm, Hasher};

use ergibus_lib::snapshot;

use crate::g_archive;
use pw_gtk_ext::glib::Value;
use pw_gtk_ext::gtkx::buffered_list_store::RowDataSource;
use pw_gtk_ext::gtkx::buffered_list_view::{BufferedListView, BufferedListViewBuilder};
use pw_gtk_ext::gtkx::dialog_user::TopGtkWindow;

#[derive(Default)]
struct SnapshotRowDataCore {
    archive_name: RefCell<Option<String>>,
    raw_row_data: RefCell<Vec<String>>,
}

#[derive(WClone, Default)]
struct SnapshotRowData(Rc<SnapshotRowDataCore>);

impl SnapshotRowData {
    fn new() -> Self {
        Self::default()
    }

    fn set_archive_name(&self, new_archive_name: Option<String>) {
        let mut archive_name = self.0.archive_name.borrow_mut();
        *archive_name = new_archive_name
    }
}

impl RowDataSource for SnapshotRowData {
    fn column_types(&self) -> Vec<glib::Type> {
        vec![glib::Type::String]
    }

    fn columns(&self) -> Vec<gtk::TreeViewColumn> {
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
        vec![col]
    }

    fn generate_rows(&self) -> Vec<Vec<Value>> {
        let mut rows = vec![];
        let raw_row_data = self.0.raw_row_data.borrow();
        for item in raw_row_data.iter() {
            rows.push(vec![item.to_value()]);
        }
        rows
    }

    fn refresh(&self) -> Vec<u8> {
        let mut raw_row_data = self.0.raw_row_data.borrow_mut();
        *raw_row_data = vec![];
        let archive_name = &*self.0.archive_name.borrow();
        match archive_name {
            Some(archive_name) => {
                match snapshot::get_snapshot_names_for_archive(&archive_name, true) {
                    Ok(snapshot_names) => {
                        let hash = generate_digest(&snapshot_names);
                        *raw_row_data = snapshot_names;
                        hash
                    }
                    Err(_) => vec![],
                }
            }
            None => vec![],
        }
    }
}

fn generate_digest(list: &Vec<String>) -> Vec<u8> {
    let mut hasher = Hasher::new(Algorithm::SHA256);
    for ref item in list {
        hasher.write_all(item.as_bytes()).expect(UNEXPECTED);
    }
    hasher.finish()
}

#[derive(PWO)]
pub struct SnapshotListViewCore {
    vbox: gtk::Box,
    archive_selector: g_archive::ArchiveSelector,
    buffered_list_view: BufferedListView<SnapshotRowData>,
    snapshot_row_data: SnapshotRowData,
}

#[derive(PWO, Wrapper, WClone)]
pub struct SnapshotListView(Rc<SnapshotListViewCore>);

impl SnapshotListView {
    pub fn new_rc() -> SnapshotListView {
        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let archive_selector = g_archive::ArchiveSelector::new();
        vbox.pack_start(&archive_selector.pwo(), false, false, 0);
        let snapshot_row_data = SnapshotRowData::new();
        let buffered_list_view = BufferedListViewBuilder::new()
            //.archive_name(archive_selector.get_selected_archive())
            .menu_item((
                "open",
                ("Open", None, Some("Open the selected snapshot")).into(),
                SAV_SELN_UNIQUE_OR_HOVER_OK,
            ))
            .build(snapshot_row_data.clone());
        let scrolled_window = gtk::ScrolledWindow::new(
            Option::<&gtk::Adjustment>::None,
            Option::<&gtk::Adjustment>::None,
        );
        scrolled_window.add(&buffered_list_view.pwo());
        vbox.pack_start(&scrolled_window, true, true, 0);
        vbox.show_all();
        let snapshot_list_view = SnapshotListView(Rc::new(SnapshotListViewCore {
            vbox,
            archive_selector,
            buffered_list_view,
            snapshot_row_data,
        }));

        let sst_c = snapshot_list_view.clone();
        snapshot_list_view
            .0
            .archive_selector
            .connect_changed(move |new_archive_name| sst_c.set_archive_name(new_archive_name));

        snapshot_list_view
    }

    pub fn set_archive_name(&self, new_archive_name: Option<String>) {
        self.0.snapshot_row_data.set_archive_name(new_archive_name);
        self.0.buffered_list_view.repopulate();
    }
}
