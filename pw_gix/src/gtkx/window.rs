// Copyright 2017 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::io::{self, Write};

use gtk;
use gtk::prelude::*;

use crate::gdkx::*;
use crate::recollections;

pub trait RememberGeometry: gtk::WidgetExt + gtk::GtkWindowExt {
    fn set_geometry_from_recollections(&self, window_name: &str, default_size: (i32, i32)) {
        let key = format!("{}::window::last_geometry", window_name);
        if let Some(last_geometry) = recollections::recall(key.as_str()) {
            if let Ok((width, height, x, y)) = parse_geometry(last_geometry.as_str()) {
                self.set_default_size(width, height);
                self.move_(x, y);
            } else {
                let msg = format!("Error parsing \"{}\"\n", key);
                io::stderr()
                    .write_all(msg.as_bytes())
                    .expect("nowhere to go");
                self.set_default_size(default_size.0, default_size.1)
            }
        } else {
            self.set_default_size(default_size.0, default_size.1)
        }
        self.connect_configure_event(move |_, event| {
            let text = format_geometry(event);
            recollections::remember(key.as_str(), text.as_str());
            false
        });
    }
}

impl RememberGeometry for gtk::ApplicationWindow {}
impl RememberGeometry for gtk::Window {}

pub trait DerivedTransientFor: gtk::GtkWindowExt {
    fn set_transient_for_from<W: gtk::WidgetExt>(&self, widget: &W) {
        if let Some(tl) = widget.get_toplevel() {
            if tl.is_toplevel() {
                if let Ok(window) = tl.dynamic_cast::<gtk::Window>() {
                    self.set_transient_for(Some(&window))
                }
            }
        }
    }

    fn set_transient_for_and_icon_from<W: gtk::WidgetExt>(&self, widget: &W) {
        if let Some(tl) = widget.get_toplevel() {
            if tl.is_toplevel() {
                if let Ok(window) = tl.dynamic_cast::<gtk::Window>() {
                    self.set_transient_for(Some(&window));
                    if let Some(ref icon) = window.get_icon() {
                        self.set_icon(Some(icon));
                    }
                }
            }
        }
    }
}

impl DerivedTransientFor for gtk::ApplicationWindow {}
impl DerivedTransientFor for gtk::Window {}
