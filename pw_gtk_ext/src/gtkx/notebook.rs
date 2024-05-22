// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::gtk::prelude::{BoxExt, ButtonExt, WidgetExt};
use std::cell::RefCell;
use std::rc::Rc;

use crate::wrapper::*;

#[derive(PWO)]
pub struct TabRemoveLabelCore {
    h_box: gtk::Box,
    remove_page_callbacks: RefCell<Vec<Box<dyn Fn()>>>,
}

#[derive(PWO, WClone)]
pub struct TabRemoveLabel(Rc<TabRemoveLabelCore>);

impl TabRemoveLabel {
    pub fn connect_remove_page<F: 'static + Fn()>(&self, callback: F) {
        self.0
            .remove_page_callbacks
            .borrow_mut()
            .push(Box::new(callback))
    }

    pub fn inform_remove_page(&self) {
        for callback in self.0.remove_page_callbacks.borrow().iter() {
            callback();
        }
    }
}

#[derive(Default)]
pub struct TabRemoveLabelBuilder {
    label_text: Option<String>,
    tooltip_text: Option<String>,
}

impl TabRemoveLabelBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn label_text(&mut self, label_text: &str) -> &mut Self {
        self.label_text = Some(label_text.to_string());
        self
    }

    pub fn tooltip_text(&mut self, tooltip_text: &str) -> &mut Self {
        self.tooltip_text = Some(tooltip_text.to_string());
        self
    }

    pub fn build(&self) -> TabRemoveLabel {
        let trl = TabRemoveLabel(Rc::new(TabRemoveLabelCore {
            h_box: gtk::Box::new(gtk::Orientation::Horizontal, 0),
            remove_page_callbacks: RefCell::new(Vec::new()),
        }));
        let label = match self.label_text {
            Some(ref text) => gtk::Label::new(Some(text)),
            None => gtk::Label::new(None),
        };
        trl.0.h_box.pack_start(&label, true, true, 0);
        let icon = gio::ThemedIcon::with_default_fallbacks("window-close-symbolic");
        let image = gtk::Image::from_gicon(&icon, gtk::IconSize::Menu);
        match self.label_text {
            Some(ref text) => image.set_tooltip_text(Some(text)),
            None => image.set_tooltip_text(None),
        };
        let button = gtk::ButtonBuilder::new()
            .relief(gtk::ReliefStyle::None)
            //.focus_on_click(false)
            .image(&image)
            .build();
        // NB: this is because rust can't find gtk::ButtonBuilder.focus_on_click()
        button.set_focus_on_click(false);
        //button.set_name("notebook-tab-remove-button");
        trl.0.h_box.pack_start(&button, false, false, 0);
        trl.0.h_box.show_all();
        let trl_c = trl.clone();
        button.connect_clicked(move |_| trl_c.inform_remove_page());

        trl
    }
}
