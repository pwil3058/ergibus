// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

//! Assist in simplifying GUI programming using the crates
//! included in the **gtk-rs** project <http://gtk-rs.org/> by providing
//! mechanisms to do common operations.

#[macro_export]
macro_rules! yield_to_pending_events {
    ( ) => {
        while gtk::events_pending() {
            gtk::main_iteration();
        }
    };
}

pub static UNEXPECTED: &str = "Unexpected error: please inform <pwil3058@bigpond.net.au>";

/// Gtk-rs components
pub use atk;
pub use cairo;
pub use gdk;
pub use gdk_pixbuf;
pub use gdkx11;
pub use gio;
pub use glib;
pub use gtk;
pub use pango;
pub use pango_sys;
pub use pangocairo;
pub use sourceview;

pub use recollections;

pub mod gdk_pixbufx;
pub mod gdkx;
pub mod geometry;
pub mod glibx;
#[macro_use]
pub mod gtkx;
pub mod printer;
pub mod sav_state;
#[macro_use]
pub mod wrapper;
pub mod sample;
