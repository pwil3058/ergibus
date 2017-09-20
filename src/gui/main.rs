extern crate gtk;
extern crate gio;

extern crate ergibus;

use gtk::prelude::*;
use gio::ApplicationExt;

use ergibus::archive;

fn activate(app: &gtk::Application) {
    let window = gtk::ApplicationWindow::new(app);
    window.set_title("ERGIBUS GUI");
    window.set_default_size(200, 200);
    let label = gtk::Label::new("GUI is under construction");
    window.add(&label);
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
