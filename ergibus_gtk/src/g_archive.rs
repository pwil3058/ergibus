use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use pw_gtk_ext::{
    glib::{Type, Value},
    gtk::{self, prelude::*},
    gtkx::combo_box_text::SortedUnique,
    gtkx::list_store::{ListRowOps, ListViewSpec, Row, WrappedListStore},
    gtkx::tree_view::{TreeViewWithPopup, TreeViewWithPopupBuilder},
    wrapper::*,
};

use ergibus_lib::archive;

#[derive(PWO)]
pub struct ArchiveSelector {
    h_box: gtk::Box,
    combo: gtk::ComboBoxText,
    changed_callbacks: RefCell<Vec<Box<dyn Fn(Option<String>)>>>,
}

impl ArchiveSelector {
    pub fn new() -> Rc<ArchiveSelector> {
        let archive_selector = Rc::new(ArchiveSelector {
            h_box: gtk::Box::new(gtk::Orientation::Horizontal, 0),
            combo: gtk::ComboBoxText::new(),
            changed_callbacks: RefCell::new(Vec::new()),
        });
        let label = gtk::Label::new(Some("Archive:")); // I18N needed here
        archive_selector.h_box.pack_start(&label, false, false, 0);
        archive_selector
            .h_box
            .pack_start(&archive_selector.combo, true, true, 5);

        let archive_selector_c = archive_selector.clone();
        archive_selector.combo.connect_changed(move |combo| {
            for callback in archive_selector_c.changed_callbacks.borrow().iter() {
                if let Some(text) = combo.get_active_text() {
                    callback(Some(String::from(text)))
                } else {
                    callback(None)
                }
            }
        });

        archive_selector.update_available_archives();

        archive_selector
    }

    pub fn get_selected_archive(&self) -> Option<String> {
        match self.combo.get_active_text() {
            Some(text) => Some(String::from(text)),
            None => None,
        }
    }

    pub fn set_selected_archive(&self, archive_name: &str) {
        self.combo.set_active_text(archive_name)
    }

    pub fn update_available_archives(&self) {
        let new_item_list = archive::get_archive_names();
        self.combo.update_with(&new_item_list);
    }

    pub fn connect_changed<F: Fn(Option<String>) + 'static>(&self, callback: F) {
        self.changed_callbacks.borrow_mut().push(Box::new(callback));
    }
}

#[derive(Default)]
struct PathBufListSpec;

impl ListViewSpec for PathBufListSpec {
    fn column_types() -> Vec<Type> {
        vec![Type::String]
    }

    fn columns() -> Vec<gtk::TreeViewColumn> {
        let mut cols = vec![];
        for (column, title) in ["Inclusions"].iter().enumerate() {
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

#[derive(PWO)]
pub struct PathBufList {
    vbox: gtk::Box,
    list_view: Rc<TreeViewWithPopup>,
    list_store: WrappedListStore<PathBufListSpec>,
    path_bufs: Vec<PathBuf>,
}

impl PathBufList {
    pub fn new() -> Self {
        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let list_store = WrappedListStore::new();
        let menu_items = vec![];
        let list_view = TreeViewWithPopupBuilder::new()
            .id_field(0)
            .selection_mode(gtk::SelectionMode::Multiple)
            .menu_items(&menu_items)
            .hover_expand(true)
            .build(&list_store);
        let scrolled_window = gtk::ScrolledWindow::new(
            Option::<&gtk::Adjustment>::None,
            Option::<&gtk::Adjustment>::None,
        );
        scrolled_window.add(list_view.pwo());
        vbox.pack_start(&scrolled_window, true, true, 0);
        vbox.show_all();
        Self {
            vbox,
            list_view,
            list_store,
            path_bufs: vec![],
        }
    }

    pub fn repopulate(&self) {
        let rows = self
            .path_bufs
            .iter()
            .map(|p| vec![p.to_string_lossy().to_value()])
            .collect::<Vec<_>>();
        self.list_store.repopulate_with(&rows);
    }

    pub fn add_path_buf(&mut self, path: &Path) {
        self.path_bufs.push(path.to_path_buf());
        self.repopulate();
    }

    pub fn connect_popup_menu_item<F: Fn(Option<Value>, Row) + 'static>(
        &self,
        name: &str,
        callback: F,
    ) {
        self.list_view.connect_popup_menu_item(name, callback)
    }
}
