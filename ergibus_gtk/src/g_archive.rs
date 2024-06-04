use std::path::{Path, PathBuf};
use std::rc::Rc;

use pw_gtk_ext::{
    glib::{Type, Value},
    gtk::{self, prelude::*},
    gtkx::list_store::{ListRowOps, ListViewSpec, Row, WrappedListStore},
    gtkx::tree_view::{TreeViewWithPopup, TreeViewWithPopupBuilder},
    wrapper::*,
};

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
pub struct SimpleList<T> {
    vbox: gtk::Box,
    list_view: Rc<TreeViewWithPopup>,
    list_store: WrappedListStore<PathBufListSpec>,
    list_items: Vec<T>,
}

impl<T> SimpleList<T> {
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
            list_items: vec![],
        }
    }

    pub fn connect_popup_menu_item<F: Fn(Option<Value>, Row) + 'static>(
        &self,
        name: &str,
        callback: F,
    ) {
        self.list_view.connect_popup_menu_item(name, callback)
    }
}

impl SimpleList<PathBuf> {
    pub fn repopulate(&self) {
        let rows = self
            .list_items
            .iter()
            .map(|p| vec![p.to_string_lossy().to_value()])
            .collect::<Vec<_>>();
        self.list_store.repopulate_with(&rows);
    }

    pub fn add_path_buf(&mut self, path: &Path) {
        self.list_items.push(path.to_path_buf());
        self.repopulate();
    }
}

impl SimpleList<String> {
    pub fn repopulate(&self) {
        let rows = self
            .list_items
            .iter()
            .map(|s| vec![s.to_value()])
            .collect::<Vec<_>>();
        self.list_store.repopulate_with(&rows);
    }

    pub fn add_string(&mut self, string: &str) {
        self.list_items.push(string.to_string());
        self.repopulate();
    }
}
