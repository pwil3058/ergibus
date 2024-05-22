// Copyright 2017 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::cell::{Cell, RefCell};
use std::convert::From;
use std::rc::Rc;

use gdk;
use gtk;
use gtk::prelude::*;

use crate::geometry::*;

pub struct XYSelection {
    drawing_area: gtk::DrawingArea,
    start_xy: Cell<Option<Point>>,
    end_xy: Cell<Option<Point>>,
    selection_made: Cell<bool>,
    selection_made_callbacks: RefCell<Vec<Box<dyn Fn()>>>,
    allocation: Cell<Size<f64>>,
}

impl XYSelection {
    pub fn in_progress(&self) -> bool {
        !(self.start_xy.get().is_none() || self.selection_made.get())
    }

    pub fn selection_made(&self) -> bool {
        self.selection_made.get()
    }

    pub fn is_drawable(&self) -> bool {
        self.start_xy.get().is_some()
    }

    pub fn reset(&self) {
        self.selection_made.set(false);
        self.start_xy.set(None);
        self.end_xy.set(None);
        //self.emit("status-changed", False)
    }

    pub fn get_selected_rectangle(&self) -> Option<Rectangle<f64>> {
        if let Some(start) = self.start_xy.get() {
            if let Some(end) = self.end_xy.get() {
                let delta = end - start;
                // width and height have to be positive
                let (x, width) = if delta.x() >= 0.0 {
                    (start.x(), delta.x())
                } else {
                    (end.x(), -delta.x())
                };
                let (y, height) = if delta.y() >= 0.0 {
                    (start.y(), delta.y())
                } else {
                    (end.y(), -delta.y())
                };
                Some(Rectangle {
                    x,
                    y,
                    width,
                    height,
                })
            } else {
                panic!("File: {:?} Line: {:?}: should NOT happen", file!(), line!());
            }
        } else {
            None
        }
    }

    pub fn connect_selection_made<F: 'static + Fn()>(&self, callback: F) {
        self.selection_made_callbacks
            .borrow_mut()
            .push(Box::new(callback))
    }
}

impl XYSelection {
    pub fn create(drawing_area: &gtk::DrawingArea) -> Rc<XYSelection> {
        let events = gdk::EventMask::POINTER_MOTION_MASK
            | gdk::EventMask::BUTTON_PRESS_MASK
            | gdk::EventMask::BUTTON_RELEASE_MASK
            | gdk::EventMask::LEAVE_NOTIFY_MASK;
        drawing_area.add_events(events);
        let allocation = Cell::new(Rectangle::from(drawing_area.get_allocation()).size());
        let xys = Rc::new(XYSelection {
            drawing_area: drawing_area.clone(),
            start_xy: Cell::new(None),
            end_xy: Cell::new(None),
            selection_made: Cell::new(false),
            selection_made_callbacks: RefCell::new(Vec::new()),
            allocation,
        });
        let xys_c = xys.clone();
        xys.drawing_area
            .connect_button_press_event(move |da, event| {
                if event.get_button() == 1
                    && !event.get_state().contains(gdk::ModifierType::CONTROL_MASK)
                {
                    let point = Point::from(event.get_position());
                    xys_c.start_xy.set(Some(point));
                    xys_c.end_xy.set(Some(point));
                    xys_c.selection_made.set(false);
                    da.queue_draw();
                    Inhibit(true)
                } else if event.get_button() == 2 {
                    if xys_c.in_progress() || xys_c.selection_made.get() {
                        xys_c.reset()
                    };
                    da.queue_draw();
                    Inhibit(true)
                } else {
                    Inhibit(false)
                }
            });
        let xys_c = xys.clone();
        xys.drawing_area
            .connect_button_release_event(move |da, event| {
                if event.get_button() == 1 && xys_c.in_progress() {
                    xys_c.end_xy.set(Some(Point::from(event.get_position())));
                    xys_c.selection_made.set(true);
                    for callback in xys_c.selection_made_callbacks.borrow().iter() {
                        callback();
                    }
                    da.queue_draw();
                    Inhibit(true)
                } else {
                    Inhibit(false)
                }
            });
        let xys_c = xys.clone();
        xys.drawing_area
            .connect_motion_notify_event(move |da, event| {
                if xys_c.in_progress() {
                    xys_c.end_xy.set(Some(Point::from(event.get_position())));
                    da.queue_draw();
                    Inhibit(true)
                } else {
                    Inhibit(false)
                }
            });
        let xys_c = xys.clone();
        xys.drawing_area.connect_leave_notify_event(move |da, _| {
            if xys_c.in_progress() {
                xys_c.reset();
                da.queue_draw();
            };
            Inhibit(false)
        });
        let xys_c = xys.clone();
        xys.drawing_area
            .connect_size_allocate(move |_da, allocation| {
                let new_size = Rectangle::from(*allocation).size();
                let cur_size = xys_c.allocation.get();
                if cur_size.width > 0.0 && cur_size.height > 0.0 {
                    let x_scale = new_size.width / cur_size.width;
                    let y_scale = new_size.height / cur_size.height;
                    if let Some(start_xy) = xys_c.start_xy.get() {
                        let new_start_xy = Point(start_xy.0 * x_scale, start_xy.1 * y_scale);
                        xys_c.start_xy.set(Some(new_start_xy));
                    }
                    if let Some(end_xy) = xys_c.end_xy.get() {
                        let new_end_xy = Point(end_xy.0 * x_scale, end_xy.1 * y_scale);
                        xys_c.end_xy.set(Some(new_end_xy));
                    }
                }
                xys_c.allocation.set(new_size);
            });

        xys
    }
}
