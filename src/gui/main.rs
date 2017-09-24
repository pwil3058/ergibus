extern crate gtk;
extern crate gio;

extern crate ergibus;

use gio::ApplicationExt;

use gtk::prelude::*;

use ergibus::gui::g_archive;

fn activate(app: &gtk::Application) {
    let window = gtk::ApplicationWindow::new(app);
    window.set_title("ERGIBUS GUI");
    window.set_default_size(200, 200);
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
