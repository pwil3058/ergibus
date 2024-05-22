// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use log;

use crate::gdkx::{format_geometry_size, parse_geometry_size};
use recollections;

fn get_dialog_size_corrn() -> (i32, i32) {
    match recollections::recall("dialog::size_correction") {
        Some(corrn) => match parse_geometry_size(corrn.as_str()) {
            Ok(size) => size,
            Err(err) => {
                log::error!("Error parsing \"dialog::size_correction\": {}", err);
                (0, 0)
            }
        },
        None => (0, 0),
    }
}

fn recall_dialog_last_size(key: &str, default: (i32, i32)) -> (i32, i32) {
    match recollections::recall(key) {
        Some(last_size) => match parse_geometry_size(last_size.as_str()) {
            Ok(last_size) => {
                let corrn = get_dialog_size_corrn();
                (last_size.0 + corrn.0, last_size.1 + corrn.1)
            }
            Err(err) => {
                log::error!("Error parsing \"{}\": {}", key, err);
                default
            }
        },
        None => {
            log::error!("{}: dialog no known to recollections", key);
            default
        }
    }
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
