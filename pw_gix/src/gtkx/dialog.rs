// Copyright 2017 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::io::{self, Write};

use gtk;

use crate::gdkx::*;
use crate::recollections;

pub use crate::gtkx::dialog_user;

fn get_dialog_size_corrn() -> (i32, i32) {
    if let Some(corrn) = recollections::recall("dialog::size_correction") {
        if let Ok((width, height)) = parse_geometry_size(corrn.as_str()) {
            return (width, height);
        } else {
            io::stderr()
                .write_all(b"Error parsing \"dialog::size_correction\"\n")
                .expect("such is life");
        }
    };
    (0, 0)
}

fn recall_dialog_last_size(key: &str, default: (i32, i32)) -> (i32, i32) {
    if let Some(last_size) = recollections::recall(key) {
        if let Ok((width, height)) = parse_geometry_size(last_size.as_str()) {
            let (w_corrn, h_corrn) = get_dialog_size_corrn();
            return (width + w_corrn, height + h_corrn);
        } else {
            let msg = format!("Error parsing \"{}\"\n", key);
            io::stderr()
                .write_all(msg.as_bytes())
                .expect("such is life");
        }
    };
    default
}

pub trait RememberDialogSize: gtk::WidgetExt + gtk::GtkWindowExt {
    fn set_size_from_recollections(&self, dialog_name: &str, default: (i32, i32)) {
        let key = format!("{}::dialog::last_size", dialog_name);
        let (width, height) = recall_dialog_last_size(key.as_str(), default);
        self.set_default_size(width, height);
        self.connect_configure_event(move |_, event| {
            let text = format_geometry_size(event);
            recollections::remember(key.as_str(), text.as_str());
            false
        });
        self.connect_realize(|widget| {
            let (req_width, req_height) = widget.get_default_size();
            let allocation = widget.get_allocation();
            let width_corrn = if req_width > 0 {
                req_width - allocation.width
            } else {
                0
            };
            let height_corrn = if req_height > 0 {
                req_height - allocation.height
            } else {
                0
            };
            let text = format!("{}x{}", width_corrn, height_corrn);
            recollections::remember("dialog::size_correction", text.as_str())
        });
    }
}

impl RememberDialogSize for gtk::Dialog {}
impl RememberDialogSize for gtk::AboutDialog {}
impl RememberDialogSize for gtk::AppChooserDialog {}
impl RememberDialogSize for gtk::ColorChooserDialog {}
impl RememberDialogSize for gtk::FileChooserDialog {}
impl RememberDialogSize for gtk::FontChooserDialog {}
impl RememberDialogSize for gtk::MessageDialog {}
impl RememberDialogSize for gtk::RecentChooserDialog {}
