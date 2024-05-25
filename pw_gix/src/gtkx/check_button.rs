// Copyright 2020 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

use gtk::prelude::*;

use crate::wrapper::*;

pub type ChangeCallback = Box<dyn Fn(Option<&str>)>;

#[derive(PWO)]
pub struct MutuallyExclusiveCheckButtons {
    box_: gtk::Box,
    check_buttons: HashMap<String, gtk::CheckButton>,
    change_callbacks: RefCell<Vec<ChangeCallback>>,
    suppress_inform: Cell<bool>,
}

impl MutuallyExclusiveCheckButtons {
    fn uncheck_except(&self, except: &str) {
        self.suppress_inform.set(true);
        for (name, check_button) in self.check_buttons.iter() {
            if name != except {
                check_button.set_active(false);
            } else {
                debug_assert!(check_button.get_active());
            }
        }
        self.suppress_inform.set(false);
        self.inform_change(Some(except))
    }

    fn inform_change(&self, selected: Option<&str>) {
        if !self.suppress_inform.get() {
            for callback in self.change_callbacks.borrow().iter() {
                callback(selected)
            }
        }
    }

    pub fn selected(&self) -> Option<&str> {
        for (name, check_button) in self.check_buttons.iter() {
            if check_button.get_active() {
                return Some(name);
            }
        }
        None
    }

    pub fn connect_changed<F: Fn(Option<&str>) + 'static>(&self, callback: F) {
        let boxed = Box::new(callback);
        self.change_callbacks.borrow_mut().push(boxed);
    }
}

pub struct MutuallyExclusiveCheckButtonsBuilder {
    check_buttons: Vec<(String, String, String)>,
    orientation: gtk::Orientation,
    spacing: i32,
}

impl MutuallyExclusiveCheckButtonsBuilder {
    pub fn new() -> MutuallyExclusiveCheckButtonsBuilder {
        MutuallyExclusiveCheckButtonsBuilder {
            check_buttons: vec![],
            orientation: gtk::Orientation::Horizontal,
            spacing: 0,
        }
    }

    pub fn orientation(mut self, orientation: gtk::Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn check_button(mut self, name: &str, label_text: &str, tooltip_text: &str) -> Self {
        self.check_buttons.push((
            name.to_string(),
            label_text.to_string(),
            tooltip_text.to_string(),
        ));
        self
    }

    pub fn build(self) -> Rc<MutuallyExclusiveCheckButtons> {
        let box_ = gtk::Box::new(self.orientation, self.spacing);
        let mut check_buttons = HashMap::new();
        for (name, label_text, tooltip_text) in self.check_buttons {
            let check_button = gtk::CheckButtonBuilder::new()
                .label(&label_text)
                .tooltip_text(&tooltip_text)
                .build();
            box_.pack_start(&check_button, false, false, 0);
            check_buttons.insert(name, check_button);
        }
        let mecb = Rc::new(MutuallyExclusiveCheckButtons {
            box_,
            check_buttons,
            change_callbacks: RefCell::new(vec![]),
            suppress_inform: Cell::new(false),
        });

        for (name, check_button) in mecb.check_buttons.iter() {
            let mecb_c = Rc::clone(&mecb);
            let except = name.to_string();
            check_button.connect_toggled(move |cb| {
                if cb.get_active() {
                    mecb_c.uncheck_except(&except);
                } else {
                    mecb_c.inform_change(None);
                }
            });
        }

        mecb
    }
}

impl Default for MutuallyExclusiveCheckButtonsBuilder {
    fn default() -> Self {
        Self::new()
    }
}
