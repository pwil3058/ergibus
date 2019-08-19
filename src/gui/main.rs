extern crate gio;
extern crate gtk;

extern crate pw_gix;

extern crate ergibus;

use gio::ApplicationExt;
use gio::ApplicationExtManual;

use gtk::prelude::*;

use pw_gix::gdkx::format_geometry;
use pw_gix::recollections;
use pw_gix::wrapper::*;

//use ergibus::gui::g_archive;
use ergibus::gui::g_snapshot;

use ergibus::config;

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
    let snapshot_selector = g_snapshot::SnapshotSelector::new_rc();
    vbox.pack_start(&snapshot_selector.pwo(), false, false, 0);
    let label = gtk::Label::new(Some("GUI is under construction"));
    vbox.pack_start(&label, true, true, 0);
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
