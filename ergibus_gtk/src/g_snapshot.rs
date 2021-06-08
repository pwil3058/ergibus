use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use pw_gtk_ext::{
    gtk::{self, prelude::*},
    wrapper::*,
    UNEXPECTED,
};

use crypto_hash::{Algorithm, Hasher};

use ergibus_lib::{snapshot, EResult};

use crate::{g_archive, icons};
use ergibus_lib::fs_objects::{DirectoryData, FileSystemObject, Name};
use ergibus_lib::snapshot::SnapshotPersistentData;
use pw_gtk_ext::glib::{Type, Value};
use pw_gtk_ext::gtk::ButtonBuilder;
use pw_gtk_ext::gtkx::buffered_list_store::RowDataSource;
use pw_gtk_ext::gtkx::buffered_list_view::{BufferedListView, BufferedListViewBuilder};
use pw_gtk_ext::gtkx::dialog_user::TopGtkWindow;
use pw_gtk_ext::gtkx::list_store::{ListRowOps, ListViewSpec, WrappedListStore};
use pw_gtk_ext::gtkx::list_view::{ListView, ListViewBuilder};
use pw_gtk_ext::gtkx::menu::MenuItemSpec;
use pw_gtk_ext::gtkx::notebook::TabRemoveLabelBuilder;
use pw_gtk_ext::gtkx::paned::RememberPosition;
use pw_gtk_ext::gtkx::tree_view::{TreeViewWithPopup, TreeViewWithPopupBuilder};
use pw_gtk_ext::sav_state::{SAV_SELN_MADE, SAV_SELN_UNIQUE_OR_HOVER_OK};
use std::path::{Path, PathBuf};

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

impl ListViewSpec for SnapshotRowData {
    fn column_types() -> Vec<Type> {
        vec![Type::String]
    }

    fn columns() -> Vec<gtk::TreeViewColumn> {
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
}

impl RowDataSource for SnapshotRowData {
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
    changed_archive_callbacks: RefCell<Vec<Box<dyn Fn(Option<String>)>>>,
}

#[derive(PWO, Wrapper, WClone)]
pub struct SnapshotListView(Rc<SnapshotListViewCore>);

impl SnapshotListView {
    pub fn archive_name(&self) -> Option<String> {
        self.0.snapshot_row_data.0.archive_name.borrow().clone()
    }

    pub fn set_archive_name(&self, archive_name: Option<String>) {
        if archive_name != self.archive_name() {
            self.0
                .snapshot_row_data
                .set_archive_name(archive_name.clone());
            self.0.buffered_list_view.repopulate();
            for callback in self.0.changed_archive_callbacks.borrow().iter() {
                callback(archive_name.clone())
            }
        }
    }

    pub fn repopulate(&self) {
        self.0.buffered_list_view.repopulate()
    }

    pub fn update(&self) {
        self.0.buffered_list_view.update()
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
            changed_archive_callbacks: RefCell::new(vec![]),
        }));

        let sst_c = snapshot_list_view.clone();
        snapshot_list_view
            .0
            .archive_selector
            .connect_changed(move |archive_name| sst_c.set_archive_name(archive_name));

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

#[derive(PWO, WClone, Wrapper)]
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
            .menu_item((
                "delete",
                ("Delete", None, Some("Delete the selected snapshot(s).")).into(),
                SAV_SELN_MADE,
            ))
            .build();
        paned.add1(&snapshot_list_view.pwo());
        let notebook = gtk::NotebookBuilder::new()
            .scrollable(true)
            .enable_popup(true)
            .build();
        paned.add2(&notebook);
        let snapshots_mgr = Self(Rc::new(SnapshotsManagerCore {
            paned,
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
                snapshots_mgr_clone.open_snapshot(&snapshot_name);
            },
        );

        let snapshots_mgr_clone = snapshots_mgr.clone();
        snapshots_mgr
            .0
            .snapshot_list_view
            .connect_double_click(move |value| {
                let snapshot_name = value.get::<String>().expect(UNEXPECTED).expect(UNEXPECTED);
                snapshots_mgr_clone.open_snapshot(&snapshot_name);
            });

        let snapshots_mgr_clone = snapshots_mgr.clone();
        snapshots_mgr
            .0
            .snapshot_list_view
            .connect_popup_menu_item("delete", move |_, selected| {
                let snapshot_names: Vec<String> = selected
                    .iter()
                    .map(|value| value.get::<String>().expect(UNEXPECTED).expect(UNEXPECTED))
                    .collect();
                snapshots_mgr_clone.delete_snapshots(&snapshot_names);
            });

        let snapshots_mgr_clone = snapshots_mgr.clone();
        snapshots_mgr
            .0
            .snapshot_list_view
            .connect_archive_change(move |_| snapshots_mgr_clone.close_all_snapshots());

        snapshots_mgr
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
                match SnapshotManager::new(&archive_name, snapshot_name) {
                    Ok(page) => {
                        let tab_label = TabRemoveLabelBuilder::new()
                            .label_text(snapshot_name)
                            .build();
                        let self_clone = self.clone();
                        let sn_string = snapshot_name.to_string();
                        tab_label.connect_remove_page(move || {
                            self_clone.close_snapshot(&sn_string, false)
                        });
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
                    Err(err) => self.report_error(
                        &format!(
                            "Error opening \"{}\" snapshot \"{}\"",
                            archive_name, snapshot_name
                        ),
                        &err,
                    ),
                }
            }
        }
    }

    fn close_snapshot(&self, snapshot_name: &str, conditional: bool) {
        let mut open_snapshots = self.0.open_snapshots.borrow_mut();
        match open_snapshots.binary_search_by_key(&snapshot_name, |os| os.0.as_str()) {
            Ok(index) => {
                let (_, ref page) = open_snapshots[index];
                let page_no = self.0.notebook.page_num(&page.pwo());
                self.0.notebook.remove_page(page_no);
                open_snapshots.remove(index);
            }
            Err(_) => {
                if !conditional {
                    let archive_name = self.0.snapshot_list_view.archive_name().expect(UNEXPECTED);
                    log::error!(
                        "Close \"{}:{}\" failed.  Not open",
                        archive_name,
                        snapshot_name
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

    fn delete_snapshots(&self, snapshot_names: &[String]) {
        let archive_name = self.0.snapshot_list_view.archive_name().expect(UNEXPECTED);
        let mut question = "Delete the following snapshots:\n".to_string();
        for snapshot_name in snapshot_names.iter() {
            question += format!("\t{}\n", snapshot_name).as_str();
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

#[derive(PWO)]
pub struct CurrentDirectoryManagerCore {
    h_box: gtk::Box,
    button: gtk::Button,
    label: gtk::Label,
}

#[derive(PWO, WClone)]
pub struct CurrentDirectoryManager(Rc<CurrentDirectoryManagerCore>);

impl CurrentDirectoryManager {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let h_box = gtk::BoxBuilder::new()
            .orientation(gtk::Orientation::Horizontal)
            .build();
        let button = ButtonBuilder::new()
            .tooltip_text("Change directory up one level")
            .image(&icons::up_dir::sized_image_or(16).upcast::<gtk::Widget>())
            .sensitive(false)
            .build();
        let label = gtk::LabelBuilder::new()
            .halign(gtk::Align::Start)
            .xalign(0.0)
            .build();
        h_box.pack_start(&button, false, false, 0);
        h_box.pack_start(&label, true, true, 0);
        let cdm = Self(Rc::new(CurrentDirectoryManagerCore {
            h_box,
            button,
            label,
        }));
        cdm.set_curr_dir_path(path);

        cdm
    }

    pub fn set_curr_dir_path<P: AsRef<Path>>(&self, path: P) {
        self.0
            .label
            .set_text(&format!("{}", path.as_ref().display()))
    }

    pub fn set_sensitive(&self, sensitive: bool) {
        self.0.button.set_sensitive(sensitive)
    }

    pub fn connect_button_clicked<F: Fn(&gtk::Button) + 'static>(&self, f: F) {
        self.0.button.connect_clicked(f);
    }
}

#[derive(PWO)]
pub struct SnapshotManagerCore {
    v_box: gtk::Box,
    list_view: TreeViewWithPopup,
    list_store: WrappedListStore<SnapshotManagerSpec>,
    snapshot: SnapshotPersistentData,
    current_directory_manager: CurrentDirectoryManager,
    curr_dir_path: RefCell<PathBuf>,
}

#[derive(PWO, WClone)]
pub struct SnapshotManager(Rc<SnapshotManagerCore>);

#[derive(Default)]
struct SnapshotManagerSpec;

impl ListViewSpec for SnapshotManagerSpec {
    fn column_types() -> Vec<Type> {
        vec![Type::U32, Type::String]
    }

    fn columns() -> Vec<gtk::TreeViewColumn> {
        let col = gtk::TreeViewColumnBuilder::new()
            .title("Name")
            .expand(false)
            .resizable(false)
            .build();

        let cell = gtk::CellRendererTextBuilder::new()
            .editable(false)
            .xalign(0.0)
            .build();

        col.pack_start(&cell, false);
        col.add_attribute(&cell, "text", 1);
        vec![col]
    }
}

impl SnapshotManager {
    fn new(archive_name: &str, snapshot_name: &str) -> EResult<Self> {
        let snapshot = snapshot::get_named_snapshot(archive_name, snapshot_name)?;
        let base_dir_path = snapshot.base_dir_path().to_path_buf();
        let current_directory_manager = CurrentDirectoryManager::new(&base_dir_path);
        let v_box = gtk::BoxBuilder::new()
            .orientation(gtk::Orientation::Vertical)
            .build();
        v_box.pack_start(&current_directory_manager.pwo(), false, false, 0);
        let list_store = WrappedListStore::<SnapshotManagerSpec>::new();
        let list_view = TreeViewWithPopupBuilder::new()
            .enable_grid_lines(gtk::TreeViewGridLines::Horizontal)
            .width_request(640)
            .build(&list_store);
        let scrolled_window = gtk::ScrolledWindow::new(
            Option::<&gtk::Adjustment>::None,
            Option::<&gtk::Adjustment>::None,
        );
        scrolled_window.add(&list_view.pwo());
        v_box.pack_start(&scrolled_window, true, true, 0);
        v_box.show_all();
        let snapshot_manager = Self(Rc::new(SnapshotManagerCore {
            v_box,
            list_view,
            list_store,
            snapshot,
            curr_dir_path: RefCell::new(base_dir_path.clone()),
            current_directory_manager,
        }));
        snapshot_manager.set_curr_dir_path(&base_dir_path);
        snapshot_manager.repopulate();

        let snapshot_manager_clone = snapshot_manager.clone();
        snapshot_manager.0.list_view.connect_double_click(move |v| {
            snapshot_manager_clone.process_double_click(v);
        });

        let snapshot_manager_clone = snapshot_manager.clone();
        snapshot_manager
            .0
            .current_directory_manager
            .connect_button_clicked(move |_| snapshot_manager_clone.change_dir_to_parent());

        Ok(snapshot_manager)
    }

    pub fn repopulate(&self) {
        let curr_dir_path = self.0.curr_dir_path.borrow();
        let curr_dir = self
            .0
            .snapshot
            .find_subdir(&*curr_dir_path)
            .expect(UNEXPECTED);
        let rows: Vec<Vec<Value>> = curr_dir
            .contents()
            .enumerate()
            .map(|(u, s)| vec![(u as u32).to_value(), s.name().to_string_lossy().to_value()])
            .collect();
        self.0.list_store.repopulate_with(&rows);
    }

    fn curr_dir(&self) -> &DirectoryData {
        let curr_dir_path = self.0.curr_dir_path.borrow();
        self.0
            .snapshot
            .find_subdir(&*curr_dir_path)
            .expect(UNEXPECTED)
    }

    fn set_curr_dir_path<P: AsRef<Path>>(&self, path: P) {
        let mut curr_dir_path = self.0.curr_dir_path.borrow_mut();
        *curr_dir_path = path.as_ref().to_path_buf();
        self.0.current_directory_manager.set_curr_dir_path(path);
        self.0
            .current_directory_manager
            .set_sensitive(*curr_dir_path != self.0.snapshot.root_dir_path());
    }

    fn change_dir_to_parent(&self) {
        let curr_dir_path = self.0.curr_dir_path.borrow().to_owned();
        if let Some(parent_dir_path) = curr_dir_path.parent() {
            self.set_curr_dir_path(parent_dir_path);
            self.repopulate();
        }
    }

    pub fn process_double_click(&self, value: &Value) {
        let index = value.get::<u32>().expect(UNEXPECTED).expect(UNEXPECTED) as usize;
        let curr_dir = self.curr_dir();
        match curr_dir[index] {
            FileSystemObject::Directory(ref dir_data) => {
                self.set_curr_dir_path(dir_data.path());
                self.repopulate();
            }
            _ => (),
        }
    }
}
