// Copyright 2017 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>
use std::{error, fmt};

use gtk;
use gtk::prelude::{ComboBoxExt, ComboBoxExtManual, ComboBoxTextExt, TreeModelExt};

#[derive(Debug)]
pub enum Error {
    DuplicateItem(String),
    UnknownItem(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::DuplicateItem(item) => write!(f, "Duplicate item: {}", item),
            Error::UnknownItem(item) => write!(f, "Unknown item: {}", item),
        }
    }
}

impl error::Error for Error {}

pub trait SortedUnique {
    fn get_item_index(&self, item: &str) -> Result<i32, i32>;
    fn get_text_items(&self) -> Vec<String>;
    fn remove_text_item(&self, item: &str) -> Result<(), Error>;
    fn insert_text_item(&self, item: &str) -> Result<i32, Error>;
    fn set_active_text(&self, item: &str) -> Result<(), Error>;

    fn update_with(&self, new_item_list: &Vec<String>) {
        let current_item_list = self.get_text_items();
        for item in &current_item_list {
            if !new_item_list.contains(item) {
                self.remove_text_item(item).expect("it's there");
            }
        }
        for item in new_item_list {
            if !current_item_list.contains(item) {
                self.insert_text_item(item).expect("uniqueness checked");
            }
        }
    }
}

impl SortedUnique for gtk::ComboBoxText {
    fn get_item_index(&self, item: &str) -> Result<i32, i32> {
        if let Some(model) = self.get_model() {
            if let Some(ref iter) = model.get_iter_first() {
                for index in 0.. {
                    if let Some(ref text) = model
                        .get_value(iter, 0)
                        .get::<String>()
                        .expect("only using strings")
                    {
                        if text == item {
                            return Ok(index);
                        } else if item < text.as_str() {
                            return Err(index);
                        }
                    };
                    if !model.iter_next(iter) {
                        return Err(-1);
                    };
                }
            }
        };
        Err(-1)
    }

    fn get_text_items(&self) -> Vec<String> {
        let mut text_items = Vec::new();
        if let Some(model) = self.get_model() {
            if let Some(ref iter) = model.get_iter_first() {
                loop {
                    if let Some(ref text) = model
                        .get_value(iter, 0)
                        .get::<String>()
                        .expect("only using strings")
                    {
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

    fn remove_text_item(&self, item: &str) -> Result<(), Error> {
        match self.get_item_index(item) {
            Ok(index) => {
                self.remove(index);
                Ok(())
            }
            Err(_) => Err(Error::UnknownItem(item.to_string())),
        }
    }

    fn insert_text_item(&self, item: &str) -> Result<i32, Error> {
        match self.get_item_index(item) {
            Ok(_) => Err(Error::DuplicateItem(item.to_string())),
            Err(index) => {
                self.insert_text(index, item);
                Ok(index)
            }
        }
    }

    fn set_active_text(&self, item: &str) -> Result<(), Error> {
        match self.get_item_index(item) {
            Ok(index) => {
                self.set_active(Some(index as u32));
                Ok(())
            }
            Err(_) => Err(Error::UnknownItem(item.to_string())),
        }
    }
}
