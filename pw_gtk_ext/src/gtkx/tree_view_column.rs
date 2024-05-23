// Copyright 2017 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use gtk;
use gtk::prelude::*;

pub fn simple_text_column(
    title: &str,
    text_id: i32,
    sort_id: i32,
    bg_id: i32,
    fg_id: i32,
    fixed_width: i32,
    resizable: bool,
) -> gtk::TreeViewColumn {
    let col = gtk::TreeViewColumn::new();
    col.set_title(title);
    col.set_resizable(resizable);
    if sort_id >= 0 {
        col.set_sort_column_id(sort_id)
    };
    col.set_fixed_width(fixed_width);
    let cell = gtk::CellRendererText::new();
    cell.set_property_editable(false);
    col.pack_start(&cell, resizable);
    if text_id >= 0 {
        col.add_attribute(&cell, "text", text_id)
    };
    if bg_id >= 0 {
        col.add_attribute(&cell, "background-rgba", bg_id)
    };
    if fg_id >= 0 {
        col.add_attribute(&cell, "foreground-rgba", fg_id)
    };
    col
}

#[allow(clippy::too_many_arguments)]
pub fn editable_text_column<F: Fn(&gtk::CellRendererText, gtk::TreePath, &str) + 'static>(
    title: &str,
    text_id: i32,
    sort_id: i32,
    bg_id: i32,
    fg_id: i32,
    fixed_width: i32,
    resizable: bool,
    callback: F,
) -> gtk::TreeViewColumn {
    let col = gtk::TreeViewColumn::new();
    col.set_title(title);
    col.set_resizable(resizable);
    if sort_id >= 0 {
        col.set_sort_column_id(sort_id)
    };
    col.set_fixed_width(fixed_width);
    let cell = gtk::CellRendererText::new();
    cell.set_property_editable(true);
    cell.connect_edited(callback);
    col.pack_start(&cell, resizable);
    if text_id >= 0 {
        col.add_attribute(&cell, "text", text_id)
    };
    if bg_id >= 0 {
        col.add_attribute(&cell, "background-rgba", bg_id)
    };
    if fg_id >= 0 {
        col.add_attribute(&cell, "foreground-rgba", fg_id)
    };
    col
}
