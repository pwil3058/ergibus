// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::{cell::RefCell, collections::HashMap, hash::Hash, rc::Rc};

use crate::gtk::prelude::*;
use crate::wrapper::*;

pub type ChangeCallback<T> = Box<dyn Fn(&T)>;

#[derive(PWO)]
pub struct RadioButtonsCore<T: Clone + Hash> {
    box_: gtk::Box,
    radio_buttons: HashMap<T, gtk::RadioButton>,
    change_callbacks: RefCell<Vec<ChangeCallback<T>>>,
}

#[derive(PWO, WClone)]
pub struct RadioButtons<T: Clone + Hash>(Rc<RadioButtonsCore<T>>);

impl<T: Clone + Hash> RadioButtons<T> {
    fn inform_change(&self, selected: &T) {
        for callback in self.0.change_callbacks.borrow().iter() {
            callback(selected)
        }
    }

    pub fn selected(&self) -> &T {
        self.0
            .radio_buttons
            .iter()
            .filter_map(|(tag, button)| if button.get_active() { Some(tag) } else { None })
            .next()
            .expect("exactly one should be active")
    }

    pub fn connect_changed<F: Fn(&T) + 'static>(&self, callback: F) {
        let boxed = Box::new(callback);
        self.0.change_callbacks.borrow_mut().push(boxed);
    }
}

pub struct RadioButtonsBuilder<T: Clone + Hash> {
    radio_buttons: Vec<(T, &'static str, &'static str)>,
    orientation: gtk::Orientation,
    default: Option<T>,
    spacing: i32,
}

impl<T: Clone + Hash + Eq + 'static> RadioButtonsBuilder<T> {
    pub fn new() -> RadioButtonsBuilder<T> {
        RadioButtonsBuilder {
            radio_buttons: vec![],
            orientation: gtk::Orientation::Horizontal,
            spacing: 0,
            default: None,
        }
    }

    pub fn orientation(mut self, orientation: gtk::Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn radio_button(
        mut self,
        tag: T,
        label_text: &'static str,
        tooltip_text: &'static str,
    ) -> Self {
        self.radio_buttons.push((tag, label_text, tooltip_text));
        self
    }

    pub fn spacing(mut self, spacing: i32) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn default(mut self, default: T) -> Self {
        self.default = Some(default);
        self
    }

    pub fn build(self) -> RadioButtons<T> {
        let box_ = gtk::Box::new(self.orientation, self.spacing);
        let mut radio_buttons = HashMap::new();
        let mut group: Option<gtk::RadioButton> = None;
        for (tag, label_text, tooltip_text) in self.radio_buttons {
            let radio_button = gtk::RadioButtonBuilder::new()
                .label(label_text)
                .tooltip_text(tooltip_text)
                .build();
            box_.pack_start(&radio_button, false, false, 0);
            if let Some(ref group) = group {
                radio_button.join_group(Some(group));
            } else {
                group = Some(radio_button.clone());
            }
            let _result = radio_buttons.insert(tag, radio_button);
            debug_assert!(_result.is_none(), "Duplicate check button tag");
        }
        if let Some(default) = self.default {
            radio_buttons.get(&default).unwrap().set_active(true);
        }
        let radio_buttons = RadioButtons(Rc::new(RadioButtonsCore {
            box_,
            radio_buttons,
            change_callbacks: RefCell::new(vec![]),
        }));

        for (tag, radio_button) in radio_buttons.0.radio_buttons.iter() {
            let radio_buttons_c = radio_buttons.clone();
            let tag_c = tag.clone();
            radio_button.connect_toggled(move |cb| {
                if cb.get_active() {
                    radio_buttons_c.inform_change(&tag_c);
                }
            });
        }

        radio_buttons
    }
}

impl<T: Clone + Hash + Eq + 'static> Default for RadioButtonsBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}
