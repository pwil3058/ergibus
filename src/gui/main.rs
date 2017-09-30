extern crate gtk;
extern crate gio;

extern crate pw_gix;

extern crate ergibus;

use gio::ApplicationExt;

use gtk::prelude::*;

use pw_gix::gdkx::format_geometry;

use ergibus::gui::g_archive;
use ergibus::gui::recollections;

fn activate(app: &gtk::Application) {
    let window = gtk::ApplicationWindow::new(app);
    window.set_title("ERGIBUS GUI");
    if let Some(geometry) = recollections().recall("main_window:geometry") {
        window.parse_geometry(&geometry);
    } else {
        window.set_default_size(200, 200);
    };
    window.connect_configure_event(
        |_, event| {
            recollections().remember("main_window:geometry", &format_geometry(event));
            false
        }
    );
    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let archive_selector = g_archive::ArchiveSelector::new();
    archive_selector.update_contents();
    vbox.pack_start(&archive_selector.hbox, false, false, 0);
    let label = gtk::Label::new("GUI is under construction");
    vbox.pack_start(&label, true, true, 0);
    window.add(&vbox);
    window.show_all();
}

fn main() {
    let flags = gio::ApplicationFlags::empty();
    let app = gtk::Application::new("gergibus.pw.nest", flags).unwrap_or_else(
        |err| panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
    );
    app.connect_activate(activate);
    app.run(&[]);
}
