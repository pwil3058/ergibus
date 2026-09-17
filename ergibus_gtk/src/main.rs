// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use gtk3_ext::{
    gdkx::{format_geometry, parse_geometry},
    gio::{self, prelude::ApplicationExtManual},
    gtk::{self, prelude::*},
    wrapper::*,
};

use crate::g_snapshots::SnapshotsManager;
use ergibus_lib::config;

pub mod g_archive;
pub mod g_snapshot;
pub mod g_snapshots;
mod icons;

fn activate(app: &gtk::Application) {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title(("ERGIBUS GUI"))
        .build();

    let snapshots_manager = SnapshotsManager::new();
    window.add(snapshots_manager.pwo());

    if let Some(geometry_str) = recollections::recall("main_window:geometry")
        && let Ok((w, h, x, y)) = parse_geometry(&geometry_str)
    {
        window.resize(w, h);
        window.move_(x, y);
    } else {
        window.set_default_size(200, 200);
    };
    window.connect_configure_event(|_, event| {
        recollections::remember("main_window:geometry", &format_geometry(event));
        false
    });
    window.show_all();
}

fn main() {
    if let Err(err) = recollections::init(config::get_gui_config_dir_path().join("recollections")) {
        eprintln!("Failed to open recollections database: {}", err);
    };

    let flags = gio::ApplicationFlags::empty();
    let app = gtk::Application::builder()
        .application_id("org.ergibus.gui")
        .flags(flags)
        .build();
    app.connect_activate(activate);
    app.run();
}
