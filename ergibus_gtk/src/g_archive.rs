use std::cell::RefCell;
use std::rc::Rc;

use pw_gtk_ext::{
    gtk::{self, prelude::*},
    gtkx::combo_box_text::SortedUnique,
    wrapper::*,
};

use ergibus_lib::archive;

#[derive(PWO)]
pub struct ArchiveSelectorCore {
    h_box: gtk::Box,
    combo: gtk::ComboBoxText,
    changed_callbacks: RefCell<Vec<Box<dyn Fn(Option<String>)>>>,
}

#[derive(PWO, WClone)]
pub struct ArchiveSelector(Rc<ArchiveSelectorCore>);

impl ArchiveSelector {
    pub fn new() -> ArchiveSelector {
        let archive_selector = ArchiveSelector(Rc::new(ArchiveSelectorCore {
            h_box: gtk::Box::new(gtk::Orientation::Horizontal, 0),
            combo: gtk::ComboBoxText::new(),
            changed_callbacks: RefCell::new(Vec::new()),
        }));
        let label = gtk::Label::new(Some("Archive:")); // I18N needed here
        archive_selector.0.h_box.pack_start(&label, false, false, 0);
        archive_selector
            .0
            .h_box
            .pack_start(&archive_selector.0.combo, true, true, 5);

        let archive_selector_c = archive_selector.clone();
        archive_selector.0.combo.connect_changed(move |combo| {
            for callback in archive_selector_c.0.changed_callbacks.borrow().iter() {
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
        match self.0.combo.get_active_text() {
            Some(text) => Some(String::from(text)),
            None => None,
        }
    }

    pub fn set_selected_archive(&self, archive_name: &str) {
        self.0.combo.set_active_text(archive_name)
    }

    pub fn update_available_archives(&self) {
        let new_item_list = archive::get_archive_names();
        self.0.combo.update_with(&new_item_list);
    }

    pub fn connect_changed<F: Fn(Option<String>) + 'static>(&self, callback: F) {
        self.0
            .changed_callbacks
            .borrow_mut()
            .push(Box::new(callback));
    }
}
