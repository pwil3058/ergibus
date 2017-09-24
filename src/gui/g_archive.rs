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
use gtk::prelude::*;

use archive;

use gtkx::combo_box_text;
use gtkx::combo_box_text::SortedUnique;
use gtkx::combo_box_text::Updateable;

pub type ArchiveComboBox = gtk::ComboBoxText;

impl combo_box_text::Updateable for ArchiveComboBox {
    fn get_updated_item_list(&self) -> Vec<String> {
        archive::get_archive_names()
    }
}

pub struct ArchiveSelector {
    pub hbox: gtk::Box,
    // make combo "pub" as mapping connect_x() functions is to hard
    pub combo: ArchiveComboBox,
}

impl ArchiveSelector {
    pub fn new() -> ArchiveSelector {
        let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let label = gtk::Label::new("Archive:");
        hbox.pack_start(&label, false, false, 0);
        let combo = ArchiveComboBox::new();
        hbox.pack_start(&combo, true, true, 5);
        ArchiveSelector{hbox, combo}
    }

    pub fn get_selected_archive(&self) -> Option<String> {
        self.combo.get_active_text()
    }

    pub fn set_selected_archive(&self, archive_name: &str) {
        self.combo.set_active_text(archive_name)
    }

    pub fn update_contents(&self) {
        self.combo.update_contents()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {

    }
}
