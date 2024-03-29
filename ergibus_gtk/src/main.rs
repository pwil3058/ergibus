use pw_gtk_ext::{
    gdkx::format_geometry,
    gio::{self, prelude::ApplicationExtManual, ApplicationExt},
    gtk::{self, prelude::*},
    pw_recollect::recollections,
    wrapper::*,
};

use crate::g_snapshots::SnapshotsManager;
use ergibus_lib::config;

pub mod g_archive;
pub mod g_snapshot;
pub mod g_snapshots;
mod icons;

fn activate(app: &gtk::Application) {
    let window = gtk::ApplicationWindow::new(app);
    window.set_title("ERGIBUS GUI");
    if let Some(geometry) = recollections::recall("main_window:geometry") {
        window.parse_geometry(&geometry);
    } else {
        window.set_default_size(200, 200);
    };
    window.connect_configure_event(|_, event| {
        recollections::remember("main_window:geometry", &format_geometry(event));
        false
    });
    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let snapshots_manager = SnapshotsManager::new();
    vbox.pack_start(snapshots_manager.pwo(), true, true, 0);
    let label = gtk::Label::new(Some("GUI is under construction"));
    vbox.pack_start(&label, false, false, 0);
    window.add(&vbox);
    window.show_all();
}

fn main() {
    recollections::init(&config::get_gui_config_dir_path().join("recollections"));
    let flags = gio::ApplicationFlags::empty();
    let app = gtk::Application::new(None, flags)
        .unwrap_or_else(|err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err));
    app.connect_activate(activate);
    app.run(&[]);
}
