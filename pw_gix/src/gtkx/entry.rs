// Copyright 2017 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::cell::{Cell, RefCell};
use std::cmp;
use std::path::{PathBuf, MAIN_SEPARATOR};
use std::rc::Rc;

use gdk;
use gtk;
use gtk::prelude::*;

use pw_pathux;

use crate::{gtkx::list_store::*, wrapper::*};

// Labelled Text Entry

#[derive(Debug, PWO)]
pub struct LabelledTextEntry {
    h_box: gtk::Box,
    entry: gtk::Entry,
}

impl LabelledTextEntry {
    pub fn new(label: &str) -> Rc<Self> {
        let lte = Rc::new(Self {
            h_box: gtk::Box::new(gtk::Orientation::Horizontal, 0),
            entry: gtk::Entry::new(),
        });

        let label = gtk::Label::new(Some(label));
        lte.h_box.pack_start(&label, false, false, 0);
        lte.h_box.pack_start(&lte.entry, true, true, 0);

        lte
    }

    pub fn entry(&self) -> &gtk::Entry {
        &self.entry
    }
}

// Hex Entry

type ChangeCallback<U> = Box<dyn Fn(U)>;

#[derive(PWO)]
pub struct HexEntry<U>
where
    U: Default
        + Ord
        + Copy
        + num_traits_plus::NumberConstants
        + num_traits::Num
        + std::fmt::UpperHex
        + std::ops::Shr<u8, Output = U>
        + 'static,
{
    entry: gtk::Entry,
    value: Cell<U>,
    current_step: Cell<U>,
    max_step: U,
    callbacks: RefCell<Vec<ChangeCallback<U>>>,
}

impl<U> HexEntry<U>
where
    U: Default
        + Ord
        + Copy
        + num_traits_plus::NumberConstants
        + num_traits::Num
        + std::fmt::UpperHex
        + std::ops::Shr<u8, Output = U>
        + 'static,
{
    pub fn value(&self) -> U {
        self.value.get()
    }

    pub fn set_value(&self, value: U) {
        self.value.set(value);
        self.reset_entry_text();
    }

    pub fn connect_value_changed<F: 'static + Fn(U)>(&self, callback: F) {
        self.callbacks.borrow_mut().push(Box::new(callback))
    }

    fn incr_value(&self) {
        let value = self.value.get();
        let adj_incr = cmp::min(U::MAX - value, self.current_step.get());
        if adj_incr > U::zero() {
            self.set_value_and_notify(value + adj_incr);
        }
        if self.value.get() < U::MAX {
            self.bump_current_step()
        }
    }

    fn decr_value(&self) {
        let value = self.value.get();
        let adj_decr = cmp::min(value, self.current_step.get());
        if adj_decr > U::zero() {
            self.set_value_and_notify(value - adj_decr);
        }
        if self.value.get() > U::MIN {
            self.bump_current_step()
        }
    }

    fn reset_entry_text(&self) {
        self.entry.set_text(&format!(
            "{:#0width$X}",
            self.value.get(),
            width = U::BYTES * 2 + 2
        ));
    }

    fn set_value_from_text(&self, text: &str) {
        let value = if let Some(index) = text.find('x') {
            U::from_str_radix(&text[index + 1..], 16)
        } else {
            U::from_str_radix(text, 16)
        };
        if let Ok(value) = value {
            self.set_value_and_notify(value);
        } else {
            self.reset_entry_text();
        }
    }

    fn set_value_and_notify(&self, value: U) {
        self.set_value(value);
        self.inform_value_changed();
    }

    fn inform_value_changed(&self) {
        let value = self.value.get();
        for callback in self.callbacks.borrow().iter() {
            callback(value);
        }
    }

    fn bump_current_step(&self) {
        let new_step = cmp::min(self.current_step.get() + U::one(), self.max_step);
        self.current_step.set(new_step);
    }

    fn reset_current_step(&self) {
        self.current_step.set(U::one());
    }
}

#[derive(Default)]
pub struct HexEntryBuilder<U>
where
    U: Default
        + Ord
        + Copy
        + num_traits_plus::NumberConstants
        + num_traits::Num
        + std::fmt::UpperHex
        + std::ops::Shr<u8, Output = U>
        + 'static,
{
    initial_value: U,
    editable: bool,
}

impl<U> HexEntryBuilder<U>
where
    U: Default
        + Ord
        + Copy
        + num_traits_plus::NumberConstants
        + num_traits::Num
        + std::fmt::UpperHex
        + std::ops::Shr<u8, Output = U>
        + 'static,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn editable(&mut self, editable: bool) -> &mut Self {
        self.editable = editable;
        self
    }

    pub fn initial_value(&mut self, initial_value: U) -> &mut Self {
        self.initial_value = initial_value;
        self
    }

    #[allow(non_upper_case_globals)]
    pub fn build(&self) -> Rc<HexEntry<U>> {
        let entry = gtk::EntryBuilder::new()
            .width_chars(U::BYTES as i32 * 2 + 2)
            .editable(self.editable)
            .build();
        let value = Cell::new(self.initial_value);
        let max_step = cmp::max(U::MAX >> 5, U::ONE);
        let current_step = Cell::new(U::ONE);
        let callbacks: RefCell<Vec<ChangeCallback<U>>> = RefCell::new(Vec::new());
        let hex_entry = Rc::new(HexEntry {
            entry,
            value,
            max_step,
            current_step,
            callbacks,
        });
        hex_entry.reset_entry_text();

        let hex_entry_c = Rc::clone(&hex_entry);
        hex_entry
            .entry
            .connect_key_press_event(move |entry, event| {
                use gdk::keys::constants::*;

                const KEY_0: gdk::keys::Key = _0;
                const KEY_1: gdk::keys::Key = _1;
                const KEY_2: gdk::keys::Key = _2;
                const KEY_3: gdk::keys::Key = _3;
                const KEY_4: gdk::keys::Key = _4;
                const KEY_5: gdk::keys::Key = _5;
                const KEY_6: gdk::keys::Key = _6;
                const KEY_7: gdk::keys::Key = _7;
                const KEY_8: gdk::keys::Key = _8;
                const KEY_9: gdk::keys::Key = _9;
                let key = event.get_keyval();
                match key {
                    Return | Tab | ISO_Left_Tab => {
                        let text = entry.get_text();
                        if text.is_empty() {
                            hex_entry_c.reset_entry_text();
                        } else {
                            hex_entry_c.set_value_from_text(&text);
                        }
                        // NB: this will nobble the "activate" signal
                        // but let the Tab key move the focus
                        Inhibit(key == Return)
                    }
                    Up => {
                        hex_entry_c.incr_value();
                        Inhibit(true)
                    }
                    Down => {
                        hex_entry_c.decr_value();
                        Inhibit(true)
                    }
                    KEY_0 | KEY_1 | KEY_2 | KEY_3 | KEY_4 | KEY_5 | KEY_6 | KEY_7 | KEY_8
                    | KEY_9 | A | B | C | D | E | F | BackSpace | Delete | Copy | Paste | x | a
                    | b | c | d | e | f | Left | Right => Inhibit(false),
                    _ => Inhibit(true),
                }
            });

        let hex_entry_c = Rc::clone(&hex_entry);
        hex_entry.entry.connect_key_release_event(move |_, event| {
            use gdk::keys::constants::*;
            match event.get_keyval() {
                Up | Down => {
                    hex_entry_c.reset_current_step();
                    Inhibit(true)
                }
                _ => Inhibit(false),
            }
        });

        hex_entry
    }
}

// FILEPATH COMPLETION

pub trait PathCompletion: EntryExt + EditableSignals {
    fn _enable_path_completion(&self, dirs_only: bool) {
        let entry_completion = gtk::EntryCompletion::new();
        entry_completion.pack_start(&gtk::CellRendererText::new(), true);
        entry_completion.set_text_column(0);
        entry_completion.set_inline_completion(true);
        entry_completion.set_inline_selection(true);
        entry_completion.set_minimum_key_length(0);
        let list_store = gtk::ListStore::new(&[glib::Type::String]);
        entry_completion.set_model(Some(&list_store));

        self.set_completion(Some(&entry_completion));
        self.connect_changed(move |editable| {
            let dir_path_txt = {
                let text = editable.get_text();
                pw_pathux::dir_path_text(&text).to_string()
            };
            list_store.clear();
            let dir_path = pw_pathux::expand_home_dir_or_mine(&PathBuf::from(&dir_path_txt));
            let abs_dir_path = pw_pathux::absolute_path_buf(&dir_path);
            if let Ok(entries) = pw_pathux::usable_dir_entries(&abs_dir_path) {
                if dirs_only {
                    for entry in entries.iter() {
                        if !entry.is_dir() {
                            continue;
                        };
                        let text = dir_path_txt.clone() + &entry.file_name();
                        list_store.append_row(&[text.to_value()]);
                    }
                } else {
                    let msep = format!("{}", MAIN_SEPARATOR);
                    for entry in entries.iter() {
                        let text = if entry.is_dir() {
                            dir_path_txt.clone() + &entry.file_name() + &msep
                        } else {
                            dir_path_txt.clone() + &entry.file_name()
                        };
                        list_store.append_row(&[text.to_value()]);
                    }
                }
            };
        });
    }

    fn enable_dir_path_completion(&self) {
        self._enable_path_completion(true)
    }

    fn enable_file_path_completion(&self) {
        self._enable_path_completion(false)
    }
}

impl PathCompletion for gtk::Entry {}
