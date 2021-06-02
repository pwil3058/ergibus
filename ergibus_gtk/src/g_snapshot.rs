use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use pw_gtk_ext::{
    gtk::{self, prelude::*},
    wrapper::*,
    UNEXPECTED,
};

use crypto_hash::{Algorithm, Hasher};

use ergibus_lib::snapshot;

use crate::g_archive;
use pw_gtk_ext::glib::{Type, Value};
use pw_gtk_ext::gtkx::buffered_list_store::RowDataSource;
use pw_gtk_ext::gtkx::buffered_list_view::{BufferedListView, BufferedListViewBuilder};
use pw_gtk_ext::gtkx::dialog_user::TopGtkWindow;
use pw_gtk_ext::gtkx::menu::MenuItemSpec;
use pw_gtk_ext::gtkx::notebook::TabRemoveLabelBuilder;
use pw_gtk_ext::gtkx::paned::RememberPosition;
use pw_gtk_ext::sav_state::SAV_SELN_UNIQUE_OR_HOVER_OK;

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
    fn column_types(&self) -> Vec<Type> {
        vec![Type::String]
    }

    fn columns(&self) -> Vec<gtk::TreeViewColumn> {
        let col = gtk::TreeViewColumnBuilder::new()
            .title("Snapshot Time")
            .expand(false)
            .resizable(false)
            .build();

        let cell = gtk::CellRendererTextBuilder::new()
            .editable(false)
            .max_width_chars(29)
            .width_chars(29)
            .xalign(0.0)
            .build();

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
    pub fn archive_name(&self) -> Option<String> {
        self.0.snapshot_row_data.0.archive_name.borrow().clone()
    }

    pub fn set_archive_name(&self, new_archive_name: Option<String>) {
        self.0.snapshot_row_data.set_archive_name(new_archive_name);
        self.0.buffered_list_view.repopulate();
    }

    pub fn connect_popup_menu_item<F: Fn(Option<Value>, Vec<Value>) + 'static>(
        &self,
        name: &str,
        callback: F,
    ) {
        self.0
            .buffered_list_view
            .connect_popup_menu_item(name, callback)
    }
}

pub struct SnapshotListViewBuilder {
    menu_items: Vec<(&'static str, MenuItemSpec, u64)>,
    id_field: i32,
    selection_mode: gtk::SelectionMode,
}

impl Default for SnapshotListViewBuilder {
    fn default() -> Self {
        Self {
            menu_items: vec![],
            id_field: 0,
            selection_mode: gtk::SelectionMode::Single,
        }
    }
}

impl SnapshotListViewBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn menu_item(&mut self, menu_item: (&'static str, MenuItemSpec, u64)) -> &mut Self {
        self.menu_items.push(menu_item);
        self
    }

    pub fn menu_items(&mut self, menu_items: Vec<(&'static str, MenuItemSpec, u64)>) -> &mut Self {
        for menu_item in menu_items.iter() {
            self.menu_items.push(menu_item.clone());
        }
        self
    }

    pub fn id_field(&mut self, id_field: i32) -> &mut Self {
        self.id_field = id_field;
        self
    }

    pub fn selection_mode(&mut self, selection_mode: gtk::SelectionMode) -> &mut Self {
        self.selection_mode = selection_mode;
        self
    }

    pub fn build(&self) -> SnapshotListView {
        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let archive_selector = g_archive::ArchiveSelector::new();
        vbox.pack_start(&archive_selector.pwo(), false, false, 0);
        let snapshot_row_data = SnapshotRowData::new();
        let buffered_list_view = BufferedListViewBuilder::new()
            .id_field(self.id_field)
            .selection_mode(self.selection_mode)
            .menu_items(&self.menu_items)
            .hover_expand(true)
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
}

#[derive(PWO)]
pub struct SnapshotsManagerCore {
    paned: gtk::Paned,
    snapshot_list_view: SnapshotListView,
    notebook: gtk::Notebook,
    open_snapshots: RefCell<Vec<(String, SnapshotManager)>>,
}

#[derive(PWO, WClone)]
pub struct SnapshotsManager(Rc<SnapshotsManagerCore>);

impl SnapshotsManager {
    pub fn new() -> Self {
        let paned = gtk::PanedBuilder::new()
            .orientation(gtk::Orientation::Horizontal)
            .name("Snapshot Files")
            .build();
        paned.set_position_from_recollections("snapshot_manager", 168);
        let snapshot_list_view = SnapshotListViewBuilder::new()
            .selection_mode(gtk::SelectionMode::Multiple)
            .menu_item((
                "open",
                ("Open", None, Some("Open indicated/selected snapshot.")).into(),
                SAV_SELN_UNIQUE_OR_HOVER_OK,
            ))
            .build();
        paned.add1(&snapshot_list_view.pwo());
        let notebook = gtk::NotebookBuilder::new()
            .scrollable(true)
            .enable_popup(true)
            .build();
        paned.add2(&notebook);
        let snapshot_mgr = Self(Rc::new(SnapshotsManagerCore {
            paned,
            snapshot_list_view,
            notebook,
            open_snapshots: RefCell::new(vec![]),
        }));

        let snapshot_mgr_clone = snapshot_mgr.clone();
        snapshot_mgr.0.snapshot_list_view.connect_popup_menu_item(
            "open",
            move |hovered, selected| {
                let snapshot_name = match selected.first() {
                    Some(value) => value.get::<String>().expect(UNEXPECTED).expect(UNEXPECTED),
                    None => hovered
                        .expect(UNEXPECTED)
                        .get::<String>()
                        .expect(UNEXPECTED)
                        .expect(UNEXPECTED),
                };
                snapshot_mgr_clone.open_snapshot(&snapshot_name);
            },
        );

        snapshot_mgr
    }

    fn open_snapshot(&self, snapshot_name: &str) {
        let mut open_snapshots = self.0.open_snapshots.borrow_mut();
        match open_snapshots.binary_search_by_key(&snapshot_name, |os| os.0.as_str()) {
            Ok(index) => {
                // already open so just make it the current page
                let (_, ref page) = open_snapshots[index];
                let page_no = self.0.notebook.page_num(&page.pwo());
                self.0.notebook.set_current_page(page_no);
            }
            Err(index) => {
                let archive_name = self.0.snapshot_list_view.archive_name().expect(UNEXPECTED);
                let page = SnapshotManager::new(&archive_name, snapshot_name);
                let tab_label = TabRemoveLabelBuilder::new()
                    .label_text(snapshot_name)
                    .build();
                let menu_label = gtk::Label::new(Some(snapshot_name));
                let page_no = self.0.notebook.insert_page_menu(
                    &page.pwo(),
                    Some(&tab_label.pwo()),
                    Some(&menu_label),
                    Some(index as u32),
                );
                open_snapshots.insert(index, (snapshot_name.to_string(), page));
                self.0.notebook.set_current_page(Some(page_no));
                self.0.notebook.show_all();
            }
        }
    }
}

#[derive(PWO)]
pub struct SnapshotManagerCore {
    h_box: gtk::Box,
}

#[derive(PWO, WClone)]
pub struct SnapshotManager(Rc<SnapshotManagerCore>);

impl SnapshotManager {
    fn new(archive_name: &str, snapshot_name: &str) -> Self {
        let h_box = gtk::BoxBuilder::new()
            .orientation(gtk::Orientation::Horizontal)
            .build();
        let label_text = format!("{}: {}\nFile data goes here", archive_name, snapshot_name);
        let label = gtk::Label::new(Some(&label_text));
        h_box.pack_start(&label, true, true, 0);
        h_box.show_all();
        Self(Rc::new(SnapshotManagerCore { h_box }))
    }
}
