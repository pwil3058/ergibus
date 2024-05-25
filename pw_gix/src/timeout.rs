// Copyright 2019 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::{self, CheckMenuItemExt};

pub type NumberedCallbacks = Vec<(u32, Box<dyn Fn()>)>;

pub struct ControlledTimeoutCycle {
    interval_secs: Cell<u32>,
    stopped: Cell<bool>, // Help try to stop multiple timeouts being in play
    check_menu_item: gtk::CheckMenuItem,
    callbacks: RefCell<NumberedCallbacks>,
    next_cb_id: Cell<u32>,
}

impl ControlledTimeoutCycle {
    pub fn new(label: &str, active: bool, interval_secs: u32) -> Rc<Self> {
        let ct = Rc::new(Self {
            interval_secs: Cell::new(interval_secs),
            stopped: Cell::new(true),
            check_menu_item: gtk::CheckMenuItem::with_label(label),
            callbacks: RefCell::new(Vec::new()),
            next_cb_id: Cell::new(0),
        });

        let ct_clone = Rc::clone(&ct);
        ct.check_menu_item.connect_toggled(move |t| {
            if t.get_active() && ct_clone.stopped.get() {
                let interval_secs = ct_clone.interval_secs.get();
                let ct_clone_clone = Rc::clone(&ct_clone);
                glib::source::timeout_add_seconds_local(interval_secs, move || {
                    for (_, callback) in ct_clone_clone.callbacks.borrow().iter() {
                        callback()
                    }
                    ct_clone_clone
                        .stopped
                        .set(!ct_clone_clone.check_menu_item.get_active());
                    glib::Continue(!ct_clone_clone.stopped.get())
                });
                ct_clone.stopped.set(false);
            }
        });

        // NB: do this last so that call back is active
        ct.check_menu_item.set_active(active);

        ct
    }

    pub fn check_menu_item(&self) -> gtk::CheckMenuItem {
        self.check_menu_item.clone()
    }

    pub fn set_interval_secs(&self, interval_secs: u32) {
        self.interval_secs.set(interval_secs);
    }

    pub fn get_interval_secs(&self) -> u32 {
        self.interval_secs.get()
    }

    pub fn register_callback(&self, callback: Box<dyn Fn()>) -> u32 {
        let cb_id = self.next_cb_id.get();
        self.next_cb_id.set(cb_id + 1);
        self.callbacks.borrow_mut().push((cb_id, callback));
        cb_id
    }
}
