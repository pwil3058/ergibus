use std::cell::RefCell;
use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::rc::Rc;

use pw_gtk_ext::{
    gtk::{self, prelude::*},
    wrapper::*,
    UNEXPECTED,
};

use crypto_hash::{Algorithm, Hasher};
use num_format::{Locale, ToFormattedString};

use ergibus_lib::snapshot;
use ergibus_lib::snapshot::Order;

use crate::g_archive;
use crate::g_snapshot::SnapshotManager;
use pw_gtk_ext::glib::{Type, Value};
use pw_gtk_ext::gtkx::buffered_list_store::{BufferedListStore, Row, RowDataSource};
use pw_gtk_ext::gtkx::dialog_user::TopGtkWindow;
use pw_gtk_ext::gtkx::list_store::ListViewSpec;
use pw_gtk_ext::gtkx::menu::MenuItemSpec;
use pw_gtk_ext::gtkx::notebook::TabRemoveLabelBuilder;
use pw_gtk_ext::gtkx::paned::RememberPosition;
use pw_gtk_ext::gtkx::tree_view::{TreeViewWithPopup, TreeViewWithPopupBuilder};
use pw_gtk_ext::sav_state::{SAV_SELN_MADE, SAV_SELN_UNIQUE_OR_HOVER_OK};

#[derive(Default)]
struct SnapshotRowDataCore {
    archive_name: RefCell<Option<String>>,
}

#[derive(WClone, Default)]
struct SnapshotRowData(Rc<SnapshotRowDataCore>);

impl SnapshotRowData {
    fn archive_name(&self) -> Option<String> {
        self.0.archive_name.borrow().clone()
    }

    fn set_archive_name(&self, new_archive_name: Option<String>) {
        let mut archive_name = self.0.archive_name.borrow_mut();
        *archive_name = new_archive_name
    }
}

impl ListViewSpec for SnapshotRowData {
    fn column_types() -> Vec<Type> {
        vec![
            Type::String,
            Type::String,
            Type::String,
            Type::String,
            Type::String,
            Type::String,
            Type::String,
        ]
    }

    fn columns() -> Vec<gtk::TreeViewColumn> {
        let mut cols = vec![];
        for (column, title) in [
            "Snapshot Time",
            "#Files",
            "#Bytes",
            "#Stored",
            "#Dir SL",
            "#File SL",
            "Time Taken",
        ]
        .iter()
        .enumerate()
        {
            let col = gtk::TreeViewColumnBuilder::new()
                .title(title)
                .expand(false)
                .resizable(false)
                .build();

            let cell = gtk::CellRendererTextBuilder::new()
                .editable(false)
                .max_width_chars(29)
                .width_chars(29)
                .xalign(1.0)
                .build();

            col.pack_start(&cell, false);
            col.add_attribute(&cell, "text", column as i32);
            cols.push(col);
        }
        cols
    }
}

impl RowDataSource for SnapshotRowData {
    fn rows_and_digest(&self) -> (Vec<Row>, Vec<u8>) {
        let archive_name = &*self.0.archive_name.borrow();
        let mut rows = vec![];
        let mut hasher = Hasher::new(Algorithm::SHA256);
        if let Some(archive_name) = archive_name {
            if let Ok(snapshot_names) =
                snapshot::iter_snapshot_names_for_archive(archive_name, Order::Descending)
            {
                for snapshot_name in snapshot_names {
                    hasher
                        .write_all(snapshot_name.to_string_lossy().as_bytes())
                        .expect(UNEXPECTED);
                    let stats = snapshot::get_snapshot_stats(archive_name, &snapshot_name)
                        .expect("should be good");
                    rows.push(vec![
                        snapshot_name.to_string_lossy().to_value(),
                        stats
                            .file_stats
                            .file_count
                            .to_formatted_string(&Locale::en_AU)
                            .to_value(),
                        stats
                            .file_stats
                            .byte_count
                            .to_formatted_string(&Locale::en_AU)
                            .to_value(),
                        stats
                            .file_stats
                            .stored_byte_count
                            .to_formatted_string(&Locale::en_AU)
                            .to_value(),
                        format!("{}", stats.sym_link_stats.dir_sym_link_count).to_value(),
                        format!("{}", stats.sym_link_stats.file_sym_link_count).to_value(),
                        format!("{:.1?}", stats.creation_duration).to_value(),
                    ]);
                }
            }
        }
        (rows, hasher.finish())
    }

    fn digest(&self) -> Vec<u8> {
        let archive_name = &*self.0.archive_name.borrow();
        let mut hasher = Hasher::new(Algorithm::SHA256);
        if let Some(archive_name) = archive_name {
            if let Ok(snapshot_names) =
                snapshot::iter_snapshot_names_for_archive(archive_name, Order::Descending)
            {
                for snapshot_name in snapshot_names {
                    hasher
                        .write_all(snapshot_name.to_string_lossy().as_bytes())
                        .expect(UNEXPECTED);
                }
            }
        }
        hasher.finish()
    }
}

#[derive(PWO, Wrapper)]
pub struct SnapshotListViewCore {
    vbox: gtk::Box,
    buffered_list_view: Rc<TreeViewWithPopup>,
    buffered_list_store: BufferedListStore<SnapshotRowData>,
    changed_archive_callbacks: RefCell<Vec<Box<dyn Fn(Option<String>)>>>,
}

#[derive(PWO, Wrapper, WClone)]
pub struct SnapshotListView(Rc<SnapshotListViewCore>);

impl SnapshotListView {
    pub fn archive_name(&self) -> Option<String> {
        self.0.buffered_list_store.row_data_source().archive_name()
    }

    pub fn set_archive_name(&self, archive_name: Option<String>) {
        if archive_name != self.archive_name() {
            self.0
                .buffered_list_store
                .row_data_source()
                .set_archive_name(archive_name.clone());
            self.0.buffered_list_store.repopulate();
            for callback in self.0.changed_archive_callbacks.borrow().iter() {
                callback(archive_name.clone())
            }
        }
    }

    pub fn repopulate(&self) {
        self.0.buffered_list_store.repopulate()
    }

    pub fn update(&self) {
        self.0.buffered_list_store.update()
    }

    pub fn connect_popup_menu_item<F: Fn(Option<Value>, Row) + 'static>(
        &self,
        name: &str,
        callback: F,
    ) {
        self.0
            .buffered_list_view
            .connect_popup_menu_item(name, callback)
    }

    pub fn connect_archive_change<F: Fn(Option<String>) + 'static>(&self, callback: F) {
        self.0
            .changed_archive_callbacks
            .borrow_mut()
            .push(Box::new(callback));
    }

    pub fn connect_double_click<F: Fn(&Value) + 'static>(&self, callback: F) {
        self.0.buffered_list_view.connect_double_click(callback)
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
        let buffered_list_store = BufferedListStore::new(SnapshotRowData::default());
        let buffered_list_view = TreeViewWithPopupBuilder::new()
            .id_field(self.id_field)
            .selection_mode(self.selection_mode)
            .menu_items(&self.menu_items)
            .hover_expand(true)
            .build(&buffered_list_store);
        let scrolled_window = gtk::ScrolledWindow::new(
            Option::<&gtk::Adjustment>::None,
            Option::<&gtk::Adjustment>::None,
        );
        scrolled_window.add(buffered_list_view.pwo());
        vbox.pack_start(&scrolled_window, true, true, 0);
        vbox.show_all();
        let snapshot_list_view = SnapshotListView(Rc::new(SnapshotListViewCore {
            vbox,
            buffered_list_view,
            buffered_list_store,
            changed_archive_callbacks: RefCell::new(vec![]),
        }));

        snapshot_list_view
    }
}

#[derive(PWO)]
pub struct SnapshotsManagerCore {
    vbox: gtk::Box,
    archive_selector: Rc<g_archive::ArchiveSelector>,
    snapshot_list_view: SnapshotListView,
    notebook: gtk::Notebook,
    open_snapshots: RefCell<Vec<(OsString, SnapshotManager)>>,
}

#[derive(PWO, WClone, Wrapper)]
pub struct SnapshotsManager(Rc<SnapshotsManagerCore>);

impl SnapshotsManager {
    pub fn new() -> Self {
        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let new_archive_button = gtk::Button::with_label("New Archive");
        hbox.pack_start(&new_archive_button, false, false, 0);
        vbox.pack_start(&hbox, false, false, 0);
        let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let archive_selector = g_archive::ArchiveSelector::new();
        hbox.pack_start(archive_selector.pwo(), false, false, 0);
        let take_snapsot_button = gtk::Button::with_label("Take Snapshot");
        hbox.pack_start(&take_snapsot_button, false, false, 0);
        let label = gtk::Label::new(Some("Buttons go here"));
        hbox.pack_start(&label, false, false, 0);
        vbox.pack_start(&hbox, false, false, 0);
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
            .menu_item((
                "delete",
                ("Delete", None, Some("Delete the selected snapshot(s).")).into(),
                SAV_SELN_MADE,
            ))
            .build();
        vbox.pack_start(&paned, true, true, 0);
        paned.add1(snapshot_list_view.pwo());
        let notebook = gtk::NotebookBuilder::new()
            .scrollable(true)
            .enable_popup(true)
            .build();
        paned.add2(&notebook);
        let snapshots_mgr = Self(Rc::new(SnapshotsManagerCore {
            vbox,
            archive_selector,
            snapshot_list_view,
            notebook,
            open_snapshots: RefCell::new(vec![]),
        }));

        let snapshots_mgr_clone = snapshots_mgr.clone();
        snapshots_mgr.0.snapshot_list_view.connect_popup_menu_item(
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
                snapshots_mgr_clone.open_snapshot(&OsString::from(snapshot_name));
            },
        );

        let snapshots_mgr_clone = snapshots_mgr.clone();
        snapshots_mgr
            .0
            .snapshot_list_view
            .connect_double_click(move |value| {
                let snapshot_name = value.get::<String>().expect(UNEXPECTED).expect(UNEXPECTED);
                snapshots_mgr_clone.open_snapshot(&OsString::from(snapshot_name));
            });

        let snapshots_mgr_clone = snapshots_mgr.clone();
        snapshots_mgr
            .0
            .snapshot_list_view
            .connect_popup_menu_item("delete", move |_, selected| {
                let snapshot_names: Vec<OsString> = selected
                    .iter()
                    .map(|value| {
                        OsString::from(value.get::<String>().expect(UNEXPECTED).expect(UNEXPECTED))
                    })
                    .collect();
                snapshots_mgr_clone.delete_snapshots(&snapshot_names);
            });

        let snapshots_mgr_clone = snapshots_mgr.clone();
        snapshots_mgr
            .0
            .snapshot_list_view
            .connect_archive_change(move |_| snapshots_mgr_clone.close_all_snapshots());

        let slv_c = snapshots_mgr.0.snapshot_list_view.clone();
        snapshots_mgr
            .0
            .archive_selector
            .connect_changed(move |archive_name| slv_c.set_archive_name(archive_name));

        let slv_c = snapshots_mgr.0.snapshot_list_view.clone();
        take_snapsot_button.connect_clicked(move |_| {
            if let Some(archive_name) = slv_c.archive_name() {
                slv_c.show_busy();
                if snapshot::generate_snapshot(&archive_name).is_ok() {
                    slv_c.repopulate();
                }
                slv_c.unshow_busy(None);
            }
        });

        snapshots_mgr
    }

    fn open_snapshot(&self, snapshot_name: &OsStr) {
        let mut open_snapshots = self.0.open_snapshots.borrow_mut();
        match open_snapshots.binary_search_by_key(&snapshot_name, |os| os.0.as_os_str()) {
            Ok(index) => {
                // already open so just make it the current page
                let (_, ref page) = open_snapshots[index];
                let page_no = self.0.notebook.page_num(page.pwo());
                self.0.notebook.set_current_page(page_no);
            }
            Err(index) => {
                let archive_name = self.0.snapshot_list_view.archive_name().expect(UNEXPECTED);
                match SnapshotManager::new(&archive_name, snapshot_name) {
                    Ok(page) => {
                        let tab_label = TabRemoveLabelBuilder::new()
                            .label_text(&snapshot_name.to_string_lossy())
                            .build();
                        let self_clone = self.clone();
                        let sn_clone = snapshot_name.to_os_string();
                        tab_label.connect_remove_page(move || {
                            self_clone.close_snapshot(&sn_clone, false)
                        });
                        let menu_label = gtk::Label::new(Some(&snapshot_name.to_string_lossy()));
                        let page_no = self.0.notebook.insert_page_menu(
                            page.pwo(),
                            Some(tab_label.pwo()),
                            Some(&menu_label),
                            Some(index as u32),
                        );
                        open_snapshots.insert(index, (snapshot_name.to_os_string(), page));
                        self.0.notebook.set_current_page(Some(page_no));
                        self.0.notebook.show_all();
                    }
                    Err(err) => self.report_error(
                        &format!(
                            "Error opening \"{}\" snapshot \"{}\"",
                            archive_name,
                            snapshot_name.to_string_lossy()
                        ),
                        &err,
                    ),
                }
            }
        }
    }

    fn close_snapshot(&self, snapshot_name: &OsStr, conditional: bool) {
        let mut open_snapshots = self.0.open_snapshots.borrow_mut();
        match open_snapshots.binary_search_by_key(&snapshot_name, |os| os.0.as_os_str()) {
            Ok(index) => {
                let (_, ref page) = open_snapshots[index];
                let page_no = self.0.notebook.page_num(page.pwo());
                self.0.notebook.remove_page(page_no);
                open_snapshots.remove(index);
            }
            Err(_) => {
                if !conditional {
                    let archive_name = self.0.snapshot_list_view.archive_name().expect(UNEXPECTED);
                    log::error!(
                        "Close \"{}:{}\" failed.  Not open",
                        archive_name,
                        snapshot_name.to_string_lossy()
                    )
                }
            }
        }
    }

    fn close_all_snapshots(&self) {
        while let Some(page_no) = self.0.notebook.get_current_page() {
            self.0.notebook.remove_page(Some(page_no))
        }
        self.0.open_snapshots.borrow_mut().clear();
    }

    fn delete_snapshots(&self, snapshot_names: &[OsString]) {
        let archive_name = self.0.snapshot_list_view.archive_name().expect(UNEXPECTED);
        let mut question = "Delete the following snapshots:\n".to_string();
        for snapshot_name in snapshot_names.iter() {
            question += format!("\t{}\n", snapshot_name.to_string_lossy()).as_str();
        }
        question += format!("belonging to the \"{}\" archive?", archive_name).as_str();
        let dialog_builder = self.new_message_dialog_builder();
        let dialog = dialog_builder
            .buttons(gtk::ButtonsType::OkCancel)
            .message_type(gtk::MessageType::Question)
            .modal(true)
            .text(&question)
            .build();
        if dialog.run() == gtk::ResponseType::Ok {
            let cursor = self.show_busy();
            if let Err(err) = snapshot::delete_named_snapshots(&archive_name, snapshot_names) {
                let dialog = self
                    .new_message_dialog_builder()
                    .buttons(gtk::ButtonsType::Ok)
                    .message_type(gtk::MessageType::Error)
                    .modal(true)
                    .text("Delete operation failed")
                    .secondary_text(&err.to_string())
                    .build();
                dialog.run();
                dialog.close();
            } else {
                for snapshot_name in snapshot_names.iter() {
                    self.close_snapshot(snapshot_name, true)
                }
            }
            self.unshow_busy(cursor);
        }
        dialog.close();
        self.0.snapshot_list_view.update();
    }
}
