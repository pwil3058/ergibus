// Copyright 2017 Peter Williams <pwil3058@gmail.com>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use gtk;
use gtk::{ComboBoxExt, ComboBoxTextExt, TreeModelExt};

pub trait SortedUnique {
    fn get_item_index(&self, item: &str) -> (bool, i32);
    fn get_text_items(&self) -> Vec<String>;
    fn remove_text_item(&self, item: &str) -> bool;
    fn insert_text_item(&self, item: &str) -> i32;
    fn set_active_text(&self, item: &str);
}

impl SortedUnique for gtk::ComboBoxText {
    fn get_item_index(&self, item: &str) -> (bool, i32) {
        if let Some(mut model) = self.get_model(){
            if let Some(ref iter) = model.get_iter_first() {
                for index in 0.. {
                    if let Some(ref text) = model.get_value(iter, 0).get::<String>() {
                        if text == item {
                            return (true, index);
                        } else if item < text {
                            return (false, index);
                        }
                    };
                    if !model.iter_next(iter) {
                        return (false, -1);
                    };
                }
            }
        };
        return (false, -1);
    }

    fn get_text_items(&self) -> Vec<String> {
        let mut text_items = Vec::new();
        if let Some(mut model) = self.get_model(){
            if let Some(ref iter) = model.get_iter_first() {
                loop {
                    if let Some(ref text) = model.get_value(iter, 0).get::<String>() {
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
            panic!("{:?}: line {:?}: {}: items must be unique", file!(), line!(), item)
        };
        index
    }

    fn set_active_text(&self, item: &str) {
        let (found, index) = self.get_item_index(item);
        if found {
            self.set_active(index);
        } else {
            panic!("{:?}: line {:?}: {}: unknown item", file!(), line!(), item)
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use gtk;
    use gtk::{ComboBoxExt, ComboBoxTextExt};
    use gtkx::combo_box_text::SortedUnique;

    #[test]
    fn gtkx_combo_box_text_sorted_unique() {
        if !gtk::is_initialized() {
            gtk::init();
        }
        let cbt = gtk::ComboBoxText::new();
        assert!(!cbt.remove_text_item("one"));
        assert_eq!(cbt.insert_text_item("one"), -1);
        assert_eq!(cbt.insert_text_item("two"), -1);
        assert_eq!(cbt.insert_text_item("three"), 1);
        assert_eq!(cbt.insert_text_item("four"), 0);
        assert_eq!(cbt.insert_text_item("five"), 0);
        assert_eq!(cbt.insert_text_item("six"), 3);
        assert_eq!(cbt.insert_text_item("zero"), -1);
        assert!(cbt.remove_text_item("two"));
        assert!(!cbt.remove_text_item("two"));
        assert!(cbt.remove_text_item("four"));
        assert!(!cbt.remove_text_item("four"));
        assert_eq!(cbt.get_text_items(), vec![
            "five", "one", "six", "three", "zero"
        ]);
        assert_ne!(cbt.get_text_items(), vec![
            "five", "one", "six", "ten", "three", "zero"
        ]);
    }
}
