// Copyright 2017 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::glibx::GlibValueExt;
use gtk;
use gtk::prelude::{BoxExt, ComboBoxExt, ComboBoxExtManual, ComboBoxTextExt, TreeModelExt};
use pw_gtk_ext_derive::PWO;
use std::cell::RefCell;
use std::rc::Rc;

use crate::wrapper::PackableWidgetObject;

pub trait SortedUnique {
    fn get_item_index(&self, item: &str) -> (bool, i32);
    fn get_text_items(&self) -> Vec<String>;
    fn remove_text_item(&self, item: &str) -> bool;
    fn insert_text_item(&self, item: &str) -> i32;
    fn set_active_text(&self, item: &str);

    fn update_with(&self, new_item_list: &Vec<String>) {
        let current_item_list = self.get_text_items();
        for item in &current_item_list {
            if !new_item_list.contains(item) {
                self.remove_text_item(item);
            }
        }
        for item in new_item_list {
            if !current_item_list.contains(item) {
                self.insert_text_item(item);
            }
        }
    }
}

impl SortedUnique for gtk::ComboBoxText {
    fn get_item_index(&self, item: &str) -> (bool, i32) {
        if let Some(model) = self.get_model() {
            if let Some(ref iter) = model.get_iter_first() {
                for index in 0.. {
                    if let Some(ref text) = model.get_value(iter, 0).get_ok::<String>() {
                        if text == item {
                            return (true, index);
                        } else if item < text.as_str() {
                            return (false, index);
                        }
                    };
                    if !model.iter_next(iter) {
                        return (false, -1);
                    };
                }
            }
        };
        (false, -1)
    }

    fn get_text_items(&self) -> Vec<String> {
        let mut text_items = Vec::new();
        if let Some(model) = self.get_model() {
            if let Some(ref iter) = model.get_iter_first() {
                loop {
                    if let Some(ref text) = model.get_value(iter, 0).get_ok::<String>() {
                        text_items.push(text.clone());
                    };
                    if !model.iter_next(iter) {
                        break;
                    };
                }
            }
        };
        text_items
    }

    fn remove_text_item(&self, item: &str) -> bool {
        let (found, index) = self.get_item_index(item);
        if found {
            self.remove(index);
        };
        found
    }

    fn insert_text_item(&self, item: &str) -> i32 {
        let (found, index) = self.get_item_index(item);
        if !found {
            self.insert_text(index, item);
        } else {
            panic!(
                "{:?}: line {:?}: {}: items must be unique",
                file!(),
                line!(),
                item
            )
        };
        index
    }

    fn set_active_text(&self, item: &str) {
        let (found, index) = self.get_item_index(item);
        if found {
            self.set_active(Some(index as u32));
        } else {
            panic!("{:?}: line {:?}: {}: unknown item", file!(), line!(), item)
        };
    }
}

#[derive(PWO)]
pub struct NameSelector {
    h_box: gtk::Box,
    combo: gtk::ComboBoxText,
    changed_callbacks: RefCell<Vec<Box<dyn Fn(Option<String>)>>>,
    get_names: fn() -> Vec<String>,
}

impl NameSelector {
    pub fn new(label: &str, get_names: fn() -> Vec<String>) -> Rc<NameSelector> {
        let archive_selector = Rc::new(NameSelector {
            h_box: gtk::Box::new(gtk::Orientation::Horizontal, 0),
            combo: gtk::ComboBoxText::new(),
            changed_callbacks: RefCell::new(Vec::new()),
            get_names,
        });
        let label = gtk::Label::new(Some(label)); // I18N needed here
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
        let new_item_list = (self.get_names)();
        self.combo.update_with(&new_item_list);
    }

    pub fn connect_changed<F: Fn(Option<String>) + 'static>(&self, callback: F) {
        self.changed_callbacks.borrow_mut().push(Box::new(callback));
    }
}
