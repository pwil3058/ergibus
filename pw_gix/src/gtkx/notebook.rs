// Copyright 2017 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::cell::RefCell;
use std::rc::Rc;

use gio;
use gtk;
use gtk::{BoxExt, ButtonExt, ContainerExt, WidgetExt};

use crate::wrapper::*;

#[derive(PWO)]
pub struct TabRemoveLabelCore {
    hbox: gtk::Box,
    remove_page_callbacks: RefCell<Vec<Box<dyn Fn()>>>,
}

impl TabRemoveLabelCore {
    pub fn connect_remove_page<F: 'static + Fn()>(&self, callback: F) {
        self.remove_page_callbacks
            .borrow_mut()
            .push(Box::new(callback))
    }

    pub fn inform_remove_page(&self) {
        for callback in self.remove_page_callbacks.borrow().iter() {
            callback();
        }
    }
}

pub type TabRemoveLabel = Rc<TabRemoveLabelCore>;

pub trait TabRemoveLabelInterface {
    fn create(label_text: Option<&str>, tooltip_text: Option<&str>) -> TabRemoveLabel;
}

impl TabRemoveLabelInterface for TabRemoveLabel {
    fn create(label_text: Option<&str>, tooltip_text: Option<&str>) -> TabRemoveLabel {
        let trl = Rc::new(TabRemoveLabelCore {
            hbox: gtk::Box::new(gtk::Orientation::Horizontal, 0),
            remove_page_callbacks: RefCell::new(Vec::new()),
        });
        let label = gtk::Label::new(label_text);
        trl.hbox.pack_start(&label, true, true, 0);
        let button = gtk::Button::new();
        button.set_relief(gtk::ReliefStyle::None);
        button.set_focus_on_click(false);
        let icon = gio::ThemedIcon::with_default_fallbacks("window-close-symbolic");
        let image = gtk::Image::from_gicon(&icon, gtk::IconSize::Menu);
        image.set_tooltip_text(tooltip_text);
        button.add(&image);
        //button.set_name("notebook-tab-remove-button");
        trl.hbox.pack_start(&button, false, false, 0);
        trl.hbox.show_all();
        let trl_c = trl.clone();
        button.connect_clicked(move |_| trl_c.inform_remove_page());

        trl
    }
}
