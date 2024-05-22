// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use log;
use recollections;
use std::str::FromStr;

pub trait RememberPosition: gtk::WidgetExt + gtk::PanedExt {
    fn recall_last_position(&self, paned_name: &str, default: i32) -> i32 {
        let key = format!("{}::paned::last_position", paned_name);
        let key_c = key.clone();
        self.connect_property_position_notify(move |paned| {
            let position = paned.get_position();
            let text = format!("{}", position);
            recollections::remember(key_c.as_str(), text.as_str());
        });
        match recollections::recall(key.as_str()) {
            Some(last_position_str) => match i32::from_str(last_position_str.as_str()) {
                Ok(last_position) => last_position,
                Err(err) => {
                    log::error!(
                        "Recollections: error parsing \"{}\" for \"{}\": {}",
                        last_position_str,
                        key,
                        err
                    );
                    default
                }
            },
            None => {
                log::warn!("Recollections: {}: unknown", key);
                default
            }
        }
    }

    fn set_position_from_recollections(&self, paned_name: &str, default: i32) {
        let position = self.recall_last_position(paned_name, default);
        self.set_position(position);
    }
}

impl RememberPosition for gtk::Paned {}
