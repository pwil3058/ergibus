// Copyright 2017 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::io::{self, Write};
use std::str::FromStr;

use gtk;

use crate::recollections;

pub trait RememberPosition: gtk::WidgetExt + gtk::PanedExt {
    fn recall_last_position(&self, paned_name: &str, default: i32) -> i32 {
        let key = format!("{}::paned::last_position", paned_name);
        let key_c = key.clone();
        self.connect_property_position_notify(move |paned| {
            let position = paned.get_position();
            let text = format!("{}", position);
            recollections::remember(key_c.as_str(), text.as_str());
        });
        if let Some(last_position_str) = recollections::recall(key.as_str()) {
            if let Ok(last_position) = i32::from_str(last_position_str.as_str()) {
                return last_position;
            } else {
                let msg = format!("Error parsing \"{}\"\n", key);
                io::stderr().write_all(msg.as_bytes()).unwrap();
            }
        };
        default
    }

    fn set_position_from_recollections(&self, paned_name: &str, default: i32) {
        let position = self.recall_last_position(paned_name, default);
        self.set_position(position);
    }
}

impl RememberPosition for gtk::Paned {}
