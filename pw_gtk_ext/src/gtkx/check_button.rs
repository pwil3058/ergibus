// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    hash::Hash,
    rc::Rc,
};

use gtk::prelude::*;

use crate::wrapper::*;

pub type ChangeCallback<T> = Box<dyn Fn(Option<&T>)>;

#[derive(PWO)]
pub struct MutuallyExclusiveCheckButtonsCore<T: Clone + Hash> {
    box_: gtk::Box,
    check_buttons: HashMap<T, gtk::CheckButton>,
    change_callbacks: RefCell<Vec<ChangeCallback<T>>>,
    suppress_inform: Cell<bool>,
}

#[derive(PWO, WClone)]
pub struct MutuallyExclusiveCheckButtons<T: Clone + Hash>(Rc<MutuallyExclusiveCheckButtonsCore<T>>);

impl<T: Clone + Hash + Eq> MutuallyExclusiveCheckButtons<T> {
    fn uncheck_except(&self, except: &T) {
        self.0.suppress_inform.set(true);
        for (tag, check_button) in self.0.check_buttons.iter() {
            if tag != except {
                check_button.set_active(false);
            } else {
                debug_assert!(check_button.get_active());
            }
        }
        self.0.suppress_inform.set(false);
        self.inform_change(Some(except))
    }

    fn inform_change(&self, selected: Option<&T>) {
        if !self.0.suppress_inform.get() {
            for callback in self.0.change_callbacks.borrow().iter() {
                callback(selected)
            }
        }
    }

    pub fn selected(&self) -> Option<&T> {
        for (tag, check_button) in self.0.check_buttons.iter() {
            if check_button.get_active() {
                return Some(tag);
            }
        }
        None
    }

    pub fn connect_changed<F: Fn(Option<&T>) + 'static>(&self, callback: F) {
        let boxed = Box::new(callback);
        self.0.change_callbacks.borrow_mut().push(boxed);
    }
}

pub struct MutuallyExclusiveCheckButtonsBuilder<T: Clone + Hash> {
    check_buttons: Vec<(T, &'static str, &'static str)>,
    orientation: gtk::Orientation,
    spacing: i32,
}

impl<T: Clone + Hash + Eq + 'static> MutuallyExclusiveCheckButtonsBuilder<T> {
    pub fn new() -> MutuallyExclusiveCheckButtonsBuilder<T> {
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

    pub fn check_button(
        mut self,
        tag: T,
        label_text: &'static str,
        tooltip_text: &'static str,
    ) -> Self {
        self.check_buttons.push((tag, label_text, tooltip_text));
        self
    }

    pub fn spacing(mut self, spacing: i32) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn build(self) -> MutuallyExclusiveCheckButtons<T> {
        let box_ = gtk::Box::new(self.orientation, self.spacing);
        let mut check_buttons = HashMap::new();
        for (tag, label_text, tooltip_text) in self.check_buttons {
            let check_button = gtk::CheckButtonBuilder::new()
                .label(label_text)
                .tooltip_text(tooltip_text)
                .build();
            box_.pack_start(&check_button, false, false, 0);
            let _result = check_buttons.insert(tag, check_button);
            debug_assert!(_result.is_none(), "Duplicate check button tag");
        }
        let mecb = MutuallyExclusiveCheckButtons(Rc::new(MutuallyExclusiveCheckButtonsCore {
            box_,
            check_buttons,
            change_callbacks: RefCell::new(vec![]),
            suppress_inform: Cell::new(false),
        }));

        for (tag, check_button) in mecb.0.check_buttons.iter() {
            let mecb_c = mecb.clone();
            let except = tag.clone();
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

impl<T: Clone + Hash + Eq + 'static> Default for MutuallyExclusiveCheckButtonsBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}
