// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    glibx::GlibValueExt,
    gtk::{
        GtkListStoreExt, GtkMenuItemExt, TreeModelExt, TreeSelectionExt, TreeViewExt, WidgetExt,
    },
    gtkx::{
        list_store::*,
        menu::{ManagedMenu, ManagedMenuBuilder, MenuItemSpec},
    },
    sav_state::*,
    wrapper::*,
};

type PopupCallback = Box<dyn Fn(Option<String>, Option<Vec<String>>)>;

#[derive(PWO)]
pub struct ListViewWithPopUpMenu {
    view: gtk::TreeView,
    list_store: gtk::ListStore,
    selected_id: RefCell<Option<String>>,
    popup_menu: ManagedMenu,
    callbacks: RefCell<HashMap<String, Vec<PopupCallback>>>,
    id_field: i32,
}

pub trait ListViewSpec {
    fn column_types(&self) -> Vec<glib::Type>;
    fn columns(&self) -> Vec<gtk::TreeViewColumn>;
}

impl ListViewWithPopUpMenu {
    fn set_selected_id(&self, posn: (f64, f64)) {
        if let Some(location) = self.view.get_path_at_pos(posn.0 as i32, posn.1 as i32) {
            if let Some(path) = location.0 {
                if let Some(list_store) = self.view.get_model() {
                    if let Some(iter) = list_store.get_iter(&path) {
                        let value = list_store.get_value(&iter, self.id_field);
                        if let Some(string) = value.get().unwrap() {
                            *self.selected_id.borrow_mut() = Some(string);
                            self.popup_menu.update_hover_condns(true);
                            return;
                        }
                    }
                }
            }
        };
        *self.selected_id.borrow_mut() = None;
        self.popup_menu.update_hover_condns(false);
    }

    pub fn update_popup_condns(&self, changed_condns: MaskedCondns) {
        self.popup_menu.update_condns(changed_condns)
    }

    pub fn connect_popup_menu_item<F: Fn(Option<String>, Option<Vec<String>>) + 'static>(
        &self,
        name: &str,
        callback: F,
    ) {
        self.callbacks
            .borrow_mut()
            .get_mut(name)
            .expect("invalid name")
            .push(Box::new(callback));
    }

    fn menu_item_selected(&self, name: &str) {
        let hovered_id = (*self.selected_id.borrow())
            .as_ref()
            .map(|id| id.to_string());
        let selection = self.view.get_selection();
        let (tree_paths, store) = selection.get_selected_rows();
        let selected_ids: Option<Vec<String>> = if !tree_paths.is_empty() {
            let mut vector = vec![];
            for tree_path in tree_paths.iter() {
                if let Some(iter) = store.get_iter(tree_path) {
                    if let Some(id) = store
                        .get_value(&iter, self.id_field)
                        .get::<String>()
                        .unwrap()
                    {
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
        if hovered_id.is_some() || selected_ids.is_some() {
            for callback in self
                .callbacks
                .borrow()
                .get(name)
                .expect("invalid name")
                .iter()
            {
                callback(hovered_id.clone(), selected_ids.clone())
            }
        }
    }

    pub fn add_row(&self, row: &[glib::Value]) {
        self.list_store.append_row(row);
    }

    pub fn remove_row(&self, id: &str) {
        if let Some((_, iter)) = self
            .list_store
            .find_row_where(|list_store, iter| list_store.get_value(iter, 0).get_ok() == Some(id))
        {
            self.list_store.remove(&iter);
        } else {
            panic!("{}: id not found", id);
        }
    }

    pub fn remove_all(&self) {
        self.list_store.clear();
    }
}

pub struct ListViewWithPopUpMenuBuilder {
    menu_items: Vec<(&'static str, MenuItemSpec, u64)>,
    id_field: i32,
    selection_mode: gtk::SelectionMode,
}

impl Default for ListViewWithPopUpMenuBuilder {
    fn default() -> Self {
        Self {
            menu_items: vec![],
            id_field: 0,
            selection_mode: gtk::SelectionMode::None,
        }
    }
}

impl ListViewWithPopUpMenuBuilder {
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

    pub fn build(&self, spec: &impl ListViewSpec) -> Rc<ListViewWithPopUpMenu> {
        let list_store = gtk::ListStore::new(&spec.column_types());
        let view = gtk::TreeViewBuilder::new().headers_visible(true).build();
        view.set_model(Some(&list_store));
        view.get_selection().set_mode(self.selection_mode);

        for col in spec.columns() {
            view.append_column(&col);
        }

        let popup_menu = ManagedMenuBuilder::new()
            .selection(&view.get_selection())
            .build();

        let rgb_l_v = Rc::new(ListViewWithPopUpMenu {
            view,
            list_store,
            selected_id: RefCell::new(None),
            popup_menu,
            callbacks: RefCell::new(HashMap::new()),
            id_field: self.id_field,
        });

        for (name, menu_item_spec, condns) in self.menu_items.iter() {
            let rgb_l_v_c = Rc::clone(&rgb_l_v);
            let name_c = (*name).to_string();
            match rgb_l_v
                .popup_menu
                .append_item(name, menu_item_spec, *condns)
            {
                Ok(menu_item) => {
                    menu_item.connect_activate(move |_| rgb_l_v_c.menu_item_selected(&name_c))
                }
                Err(err) => panic!("Error building menu item '{}':: {}", name, err),
            };
            // rgb_l_v
            //     .popup_menu
            //     .append_item(name, menu_item_spec, *condns)
            //     .connect_activate(move |_| rgb_l_v_c.menu_item_selected(&name_c));
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
                        rgb_l_v_c.view.get_selection().unselect_all();
                        gtk::Inhibit(true)
                    }
                    3 => {
                        rgb_l_v_c.set_selected_id(event.get_position());
                        rgb_l_v_c.popup_menu.popup_at_event(event);
                        gtk::Inhibit(true)
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
