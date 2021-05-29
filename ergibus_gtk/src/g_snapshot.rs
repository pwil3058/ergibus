use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use pw_gix::{
    gdk, glib,
    gtk::{self, prelude::*},
    gtkx::list_store::*,
    wrapper::*,
};

use crypto_hash::{Algorithm, Hasher};

use ergibus_lib::snapshot;

use crate::g_archive;
use pw_gix::gtkx::menu_ng::{ManagedMenu, ManagedMenuBuilder, MenuItemSpec};
use pw_gix::sav_state::{WidgetStatesControlled, SAV_SELN_UNIQUE_OR_HOVER_OK};
use std::collections::HashMap;

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
                match snapshot::get_snapshot_names_for_archive(archive_name, true) {
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

type PopupCallback = Box<dyn Fn(Option<String>, Option<Vec<String>>)>;

#[derive(PWO, Wrapper)]
pub struct SnapshotNameTable {
    scrolled_window: gtk::ScrolledWindow,
    pub view: gtk::TreeView,
    list_store: RefCell<SnapshotNameListStore>,
    popup_menu: ManagedMenu,
    hovered_snapshot: RefCell<Option<String>>,
    callbacks: RefCell<HashMap<String, Vec<PopupCallback>>>,
}

impl SnapshotNameTable {
    pub fn set_archive(&self, archive_name: Option<String>) {
        self.list_store.borrow_mut().set_archive_name(archive_name);
    }

    fn set_hovered_snapshot(&self, posn: (f64, f64)) {
        if let Some(location) = self.view.get_path_at_pos(posn.0 as i32, posn.1 as i32) {
            if let Some(path) = location.0 {
                if let Some(list_store) = self.view.get_model() {
                    if let Some(iter) = list_store.get_iter(&path) {
                        let value = list_store.get_value(&iter, 0);
                        if let Some(string) = value.get().unwrap() {
                            *self.hovered_snapshot.borrow_mut() = Some(string);
                            self.popup_menu.update_hover_condns(true);
                            return;
                        }
                    }
                }
            }
        };
        *self.hovered_snapshot.borrow_mut() = None;
        self.popup_menu.update_hover_condns(false);
    }

    fn menu_item_selected(&self, name: &str) {
        let hovered_snapshot = if let Some(ref id) = *self.hovered_snapshot.borrow() {
            Some(id.to_string())
        } else {
            None
        };
        let selection = self.view.get_selection();
        let (tree_paths, store) = selection.get_selected_rows();
        let selected_ids: Option<Vec<String>> = if tree_paths.len() > 0 {
            let mut vector = vec![];
            for tree_path in tree_paths.iter() {
                if let Some(iter) = store.get_iter(&tree_path) {
                    if let Some(id) = store.get_value(&iter, 0).get::<String>().unwrap() {
                        vector.push(id);
                    }
                }
            }
            if vector.is_empty() {
                None
            } else {
                Some(vector)
            }
        } else {
            None
        };
        if hovered_snapshot.is_some() || selected_ids.is_some() {
            for callback in self
                .callbacks
                .borrow()
                .get(name)
                .expect("invalid name")
                .iter()
            {
                callback(hovered_snapshot.clone(), selected_ids.clone())
            }
        }
    }
}

#[derive(Default)]
pub struct SnapshotNameTableBuilder {
    menu_items: Vec<(&'static str, MenuItemSpec, u64)>,
    archive_name: Option<String>,
}

impl SnapshotNameTableBuilder {
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

    pub fn archive_name(&mut self, archive_name: Option<String>) -> &mut Self {
        self.archive_name = archive_name;
        self
    }

    pub fn build(&self) -> Rc<SnapshotNameTable> {
        let archive_name = if let Some(ref archive_name) = self.archive_name {
            Some(archive_name.to_string())
        } else {
            None
        };
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

        let scrolled_window = gtk::ScrolledWindow::new(
            Option::<&gtk::Adjustment>::None,
            Option::<&gtk::Adjustment>::None,
        );
        scrolled_window.add(&view);

        let rgb_l_v = Rc::new(SnapshotNameTable {
            scrolled_window,
            view,
            list_store,
            hovered_snapshot: RefCell::new(None),
            popup_menu: ManagedMenuBuilder::new().build(),
            callbacks: RefCell::new(HashMap::new()),
        });

        for (name, menu_item_spec, condns) in self.menu_items.iter() {
            let rgb_l_v_c = Rc::clone(&rgb_l_v);
            let name_c = (*name).to_string();
            rgb_l_v
                .popup_menu
                .append_item(name, menu_item_spec, *condns)
                .connect_activate(move |_| rgb_l_v_c.menu_item_selected(&name_c));
            rgb_l_v
                .callbacks
                .borrow_mut()
                .insert((*name).to_string(), vec![]);
        }

        let rgb_l_v_c = Rc::clone(&rgb_l_v);
        rgb_l_v.view.connect_button_press_event(move |_, event| {
            if event.get_event_type() == gdk::EventType::ButtonPress {
                match event.get_button() {
                    2 => {
                        println!("DESELECT");
                        rgb_l_v_c.view.get_selection().unselect_all();
                        gtk::Inhibit(true)
                    }
                    3 => {
                        rgb_l_v_c.set_hovered_snapshot(event.get_position());
                        rgb_l_v_c.popup_menu.popup_at_event(event);
                        return gtk::Inhibit(true);
                    }
                    _ => gtk::Inhibit(false),
                }
            } else {
                gtk::Inhibit(false)
            }
        });

        rgb_l_v
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
        let snapshot_name_table = SnapshotNameTableBuilder::new()
            .archive_name(archive_selector.get_selected_archive())
            .menu_item((
                "open",
                ("Open", None, Some("Open the selected snapshot")).into(),
                SAV_SELN_UNIQUE_OR_HOVER_OK,
            ))
            .build();
        vbox.pack_start(&snapshot_name_table.pwo(), true, true, 0);
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
