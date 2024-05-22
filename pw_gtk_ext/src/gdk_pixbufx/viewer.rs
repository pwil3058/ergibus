// Copyright 2020 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::{
    cell::{Cell, RefCell},
    path::Path,
    rc::Rc,
};

use cairo::Operator;
use gdk::prelude::GdkContextExt;
use gdk_pixbuf;
use gtk::prelude::*;

use crate::{
    geometry::{AspectRatio, Point, Rectangle, Size, SizeExt},
    gtkx::{
        drawing_area::XYSelection,
        menu::{ManagedMenu, ManagedMenuBuilder},
    },
    recollections,
    sav_state::{MaskedCondns, SAV_NEXT_CONDN},
    wrapper::*,
};

struct Zoomable {
    unzoomed: gdk_pixbuf::Pixbuf,
    zoomed: RefCell<gdk_pixbuf::Pixbuf>,
    zoom_factor: Cell<f64>,
}

impl From<&gdk_pixbuf::Pixbuf> for Zoomable {
    fn from(pixbuf: &gdk_pixbuf::Pixbuf) -> Zoomable {
        Zoomable {
            unzoomed: pixbuf.clone(),
            zoomed: RefCell::new(pixbuf.clone()),
            zoom_factor: Cell::new(1.0),
        }
    }
}

impl AspectRatio for Zoomable {
    fn aspect_ratio(&self) -> f64 {
        self.unzoomed.aspect_ratio()
    }
}

impl Zoomable {
    pub fn pixbuf(&self) -> gdk_pixbuf::Pixbuf {
        self.zoomed.borrow().clone()
    }

    pub fn subpixbuf(&self, rect: Rectangle<i32>) -> Option<gdk_pixbuf::Pixbuf> {
        self.zoomed
            .borrow()
            .new_subpixbuf(rect.x, rect.y, rect.width, rect.height)
    }

    pub fn zoom_factor(&self) -> f64 {
        self.zoom_factor.get()
    }

    pub fn zoomed_size(&self) -> Size<f64> {
        self.zoomed.borrow().size().into()
    }

    pub fn set_zoom(&self, zoom_factor: f64) {
        let new_size = self.unzoomed.size() * zoom_factor;
        if let Some(new_pixbuf) = self.unzoomed.scale_simple(
            new_size.width,
            new_size.height,
            gdk_pixbuf::InterpType::Bilinear,
        ) {
            *self.zoomed.borrow_mut() = new_pixbuf;
            self.zoom_factor.set(zoom_factor);
        } //TODO: do something about failure
    }

    pub fn set_zoomed_size(&self, new_zsize: Size<i32>) {
        assert!(self.aspect_ratio_matches_size(new_zsize.into()));
        if let Some(new_pixbuf) = self.unzoomed.scale_simple(
            new_zsize.width,
            new_zsize.height,
            gdk_pixbuf::InterpType::Bilinear,
        ) {
            *self.zoomed.borrow_mut() = new_pixbuf;
            self.zoom_factor
                .set(self.zoomed.borrow().scale_versus(&self.unzoomed));
        } //TODO: do something about failure
    }

    pub fn calc_zooms_for(&self, size: Size<f64>) -> Size<f64> {
        size.scales_versus(self.unzoomed.size().into())
    }
}

#[derive(PWO, Wrapper)]
pub struct PixbufView {
    scrolled_window: gtk::ScrolledWindow,
    drawing_area: gtk::DrawingArea,
    xy_selection: Rc<XYSelection>,
    zoomable: RefCell<Option<Zoomable>>,
    ignore_size_alloc: Cell<bool>,
    last_allocation: Cell<Size<f64>>,
    zoom_in_adj: Cell<[f64; 2]>,
    zoom_out_adj: Cell<[f64; 2]>,
    last_xy: Cell<Point>,
    doing_button_motion: Cell<bool>,
    popup_menu: ManagedMenu,
}

impl PixbufView {
    const ZOOM_FACTOR: f64 = 1.005;
    const ZOOM_IN_ADJUST: f64 = (Self::ZOOM_FACTOR - 1.0) / 2.0;
    const ZOOM_OUT_ADJUST: f64 = (1.0 / Self::ZOOM_FACTOR - 1.0) / 2.0;

    const SAV_HAS_IMAGE: u64 = SAV_NEXT_CONDN;
    const SAV_HAS_SELECTION: u64 = SAV_NEXT_CONDN << 1;

    pub fn set_pixbuf(&self, o_pixbuf: Option<&gdk_pixbuf::Pixbuf>) {
        if let Some(pixbuf) = o_pixbuf {
            self.xy_selection.reset();
            let zoomable: Zoomable = pixbuf.into();
            let alloc = self.drawing_area.get_allocation().size();
            if pixbuf.aspect_ratio_matches_size(alloc.into()) {
                zoomable.set_zoomed_size(alloc);
            } else {
                let zoom = zoomable.calc_zooms_for(alloc.into()).length_longest_side();
                zoomable.set_zoom(zoom);
            };
            *self.zoomable.borrow_mut() = Some(zoomable);
            self.resize_drawing_area();
            self.popup_menu.update_condns(MaskedCondns {
                condns: Self::SAV_HAS_IMAGE,
                mask: Self::SAV_HAS_IMAGE,
            });
        } else {
            *self.zoomable.borrow_mut() = None;
            self.popup_menu.update_condns(MaskedCondns {
                condns: 0,
                mask: Self::SAV_HAS_IMAGE,
            });
        };
        self.drawing_area.queue_draw();
    }

    fn set_pixbuf_fm_file<P: AsRef<Path>>(&self, file_path: P) -> Result<(), glib::Error> {
        let file_path = file_path.as_ref();
        let pixbuf = gdk_pixbuf::Pixbuf::from_file(file_path)?;
        self.set_pixbuf(Some(&pixbuf));
        Ok(())
    }

    fn resize_drawing_area(&self) {
        if let Some(ref zoomable) = *self.zoomable.borrow() {
            self.ignore_size_alloc.set(true);
            let new_size: Size<i32> = zoomable.zoomed_size().into();
            self.drawing_area
                .set_size_request(new_size.width, new_size.height);
            let sizediff = self.scrolled_window.get_allocation().size() - new_size;
            if sizediff.width >= 0 && sizediff.height >= 0 {
                self.scrolled_window
                    .set_policy(gtk::PolicyType::Never, gtk::PolicyType::Never)
            } else {
                self.scrolled_window
                    .set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic)
            };
            self.ignore_size_alloc.set(false);
        }
    }

    fn zoom_in(&self) {
        if let Some(ref zoomable) = *self.zoomable.borrow() {
            let current_zoom = zoomable.zoom_factor();
            zoomable.set_zoom(current_zoom * Self::ZOOM_FACTOR);
            self.resize_drawing_area();
            for (dim, o_adj) in [
                self.scrolled_window.get_hadjustment(),
                self.scrolled_window.get_vadjustment(),
            ]
            .iter()
            .enumerate()
            {
                if let Some(ref adj) = *o_adj {
                    let new_val = adj.get_value() * Self::ZOOM_FACTOR + self.zoom_in_adj.get()[dim];
                    adj.set_value(new_val);
                }
            }
        }
    }

    fn zoom_out(&self) {
        if let Some(ref zoomable) = *self.zoomable.borrow() {
            let current_zoom = zoomable.zoom_factor();
            let alloc = self.last_allocation.get();
            let min_zoom = zoomable.calc_zooms_for(alloc).length_longest_side();
            if current_zoom <= min_zoom {
                gdk::beep();
                return;
            };
            zoomable.set_zoom(current_zoom / Self::ZOOM_FACTOR);
            self.resize_drawing_area();
            for (dim, o_adj) in [
                self.scrolled_window.get_hadjustment(),
                self.scrolled_window.get_vadjustment(),
            ]
            .iter()
            .enumerate()
            {
                if let Some(ref adj) = *o_adj {
                    let new_val =
                        adj.get_value() / Self::ZOOM_FACTOR + self.zoom_out_adj.get()[dim];
                    adj.set_value(new_val.max(0.0));
                }
            }
        }
    }
}

pub struct PixbufViewBuilder {
    recollection_key: String,
    load_last_image: bool,
}

impl PixbufViewBuilder {
    pub fn new() -> Self {
        Self {
            recollection_key: "image_viewer::last_image_file".to_string(),
            load_last_image: false,
        }
    }

    pub fn recollection_key(&mut self, recollection_key: &str) -> &Self {
        self.recollection_key = recollection_key.to_string();
        self
    }

    pub fn load_last_image(&mut self, value: bool) -> &Self {
        self.load_last_image = value;
        self
    }

    pub fn build(&self) -> Rc<PixbufView> {
        let drawing_area = gtk::DrawingAreaBuilder::new().build();
        let xy_selection = XYSelection::create(&drawing_area);
        let scrolled_window = gtk::ScrolledWindowBuilder::new()
            .events(
                gdk::EventMask::POINTER_MOTION_MASK
                    | gdk::EventMask::BUTTON_PRESS_MASK
                    | gdk::EventMask::BUTTON_RELEASE_MASK
                    | gdk::EventMask::LEAVE_NOTIFY_MASK,
            )
            .child(&drawing_area)
            .build();
        let alloc: Size<f64> = drawing_area.get_allocation().size().into();

        let viewer = Rc::new(PixbufView {
            scrolled_window,
            drawing_area,
            xy_selection,
            zoomable: RefCell::new(None),
            ignore_size_alloc: Cell::new(false),
            last_allocation: Cell::new(alloc),
            zoom_in_adj: Cell::new([0.0, 0.0]),
            zoom_out_adj: Cell::new([0.0, 0.0]),
            last_xy: Cell::new(Point(0.0, 0.0)),
            doing_button_motion: Cell::new(false),
            popup_menu: ManagedMenuBuilder::new().build(),
        });

        let viewer_c = Rc::clone(&viewer);
        viewer.drawing_area.connect_draw(move |_, cairo_context| {
            if let Some(ref zoomable) = *viewer_c.zoomable.borrow() {
                cairo_context.set_source_pixbuf(&zoomable.pixbuf(), 0.0, 0.0);
                cairo_context.paint();
                if viewer_c.xy_selection.is_drawable() {
                    let rect = viewer_c.xy_selection.get_selected_rectangle().unwrap();
                    if viewer_c.xy_selection.selection_made() {
                        cairo_context.set_dash(&[], 0.0)
                    } else {
                        cairo_context.set_dash(&[3.0], 0.0)
                    };
                    cairo_context.rectangle(rect.x, rect.y, rect.width, rect.height);
                    cairo_context.set_source_rgb(0.0, 0.0, 0.0);
                    cairo_context.set_operator(Operator::Xor);
                    cairo_context.stroke();
                    viewer_c.popup_menu.update_condns(MaskedCondns {
                        condns: PixbufView::SAV_HAS_SELECTION,
                        mask: PixbufView::SAV_HAS_SELECTION,
                    });
                } else {
                    viewer_c.popup_menu.update_condns(MaskedCondns {
                        condns: 0,
                        mask: PixbufView::SAV_HAS_SELECTION,
                    });
                }
            };
            Inhibit(false)
        });

        let viewer_c = Rc::clone(&viewer);
        viewer
            .scrolled_window
            .connect_size_allocate(move |sw, allocation| {
                if viewer_c.ignore_size_alloc.get() {
                    return;
                };
                let alloc = Rectangle::<f64>::from(*allocation).size();
                let last_allocation = viewer_c.last_allocation.get();
                if last_allocation != alloc {
                    viewer_c
                        .zoom_in_adj
                        .set((alloc * PixbufView::ZOOM_IN_ADJUST).into());
                    viewer_c
                        .zoom_out_adj
                        .set((alloc * PixbufView::ZOOM_OUT_ADJUST).into());
                    viewer_c.last_allocation.set(alloc);
                    let mut resize_required = false;
                    if let Some(ref zoomable) = *viewer_c.zoomable.borrow() {
                        let delta_alloc = alloc - last_allocation;
                        let zoomed_sizediff = alloc - zoomable.zoomed_size();
                        if zoomable.aspect_ratio_matches_size(alloc)
                            && zoomed_sizediff.width.abs() < 10.0
                        {
                            // a small change and same aspect ratio
                            zoomable.set_zoomed_size(alloc.into());
                            resize_required = true;
                        } else if delta_alloc.width >= 0.0 || delta_alloc.height >= 0.0 {
                            // we're getting bigger
                            if zoomed_sizediff.width > 10.0 || zoomed_sizediff.height > 10.0 {
                                let zoom = zoomable.calc_zooms_for(alloc).length_longest_side();
                                zoomable.set_zoom(zoom);
                                resize_required = true;
                            } else if zoomed_sizediff.width < 0.0 || zoomed_sizediff.height < 0.0 {
                                sw.set_policy(
                                    gtk::PolicyType::Automatic,
                                    gtk::PolicyType::Automatic,
                                )
                            } else {
                                sw.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Never)
                            }
                        } else {
                            // we're getting smaller
                            if zoomed_sizediff.width > 10.0 || zoomed_sizediff.height > 10.0 {
                                let zoom = zoomable.calc_zooms_for(alloc).length_longest_side();
                                zoomable.set_zoom(zoom);
                                resize_required = true;
                            } else if zoomed_sizediff.width < -10.0
                                && zoomed_sizediff.height < -10.0
                            {
                                if zoomed_sizediff.width > -30.0 || zoomed_sizediff.height > -30.0 {
                                    let zoom = zoomable.calc_zooms_for(alloc).length_longest_side();
                                    zoomable.set_zoom(zoom);
                                    resize_required = true;
                                }
                            } else if zoomed_sizediff.width < 0.0 || zoomed_sizediff.height < 0.0 {
                                sw.set_policy(
                                    gtk::PolicyType::Automatic,
                                    gtk::PolicyType::Automatic,
                                )
                            } else {
                                sw.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Never)
                            }
                        }
                    };
                    if resize_required {
                        viewer_c.resize_drawing_area();
                    }
                } else {
                    viewer_c
                        .zoom_in_adj
                        .set((alloc * PixbufView::ZOOM_IN_ADJUST).into());
                    viewer_c
                        .zoom_out_adj
                        .set((alloc * PixbufView::ZOOM_OUT_ADJUST).into());
                    viewer_c.last_allocation.set(alloc);
                }
            });

        // Set zoom using scroll wheel when control key pressed
        let viewer_c = Rc::clone(&viewer);
        viewer
            .scrolled_window
            .connect_scroll_event(move |_, event| {
                if !event
                    .get_state()
                    .intersects(gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK)
                {
                    match event.get_direction() {
                        gdk::ScrollDirection::Up => {
                            viewer_c.zoom_in();
                            return Inhibit(true);
                        }
                        gdk::ScrollDirection::Down => {
                            viewer_c.zoom_in();
                            return Inhibit(true);
                        }
                        gdk::ScrollDirection::Smooth => {
                            let (_, delta_y) = event.get_delta();
                            if delta_y > 0.0 {
                                viewer_c.zoom_in();
                                return Inhibit(true);
                            } else if delta_y < 0.0 {
                                viewer_c.zoom_out();
                                return Inhibit(true);
                            }
                        }
                        _ => (),
                    }
                };
                Inhibit(false)
            });

        let viewer_c = Rc::clone(&viewer);
        viewer.xy_selection.connect_selection_made(move || {
            viewer_c.drawing_area.queue_draw();
        });

        // Set up moving image with left button and control key
        let viewer_c = Rc::clone(&viewer);
        viewer
            .scrolled_window
            .connect_button_press_event(move |_, event| {
                if event.get_button() == 1
                    && event.get_state().contains(gdk::ModifierType::CONTROL_MASK)
                {
                    viewer_c.last_xy.set(event.get_position().into());
                    viewer_c.doing_button_motion.set(true);
                    return Inhibit(true);
                } else if event.get_button() == 3 {
                    viewer_c.popup_menu.popup_at_event(event);
                    return Inhibit(true);
                };
                Inhibit(false)
            });
        let viewer_c = Rc::clone(&viewer);
        viewer
            .scrolled_window
            .connect_button_release_event(move |_, event| {
                if event.get_button() == 1 && viewer_c.doing_button_motion.get() {
                    viewer_c.doing_button_motion.set(false);
                    return Inhibit(true);
                };
                Inhibit(false)
            });
        let viewer_c = Rc::clone(&viewer);
        viewer
            .scrolled_window
            .connect_leave_notify_event(move |_, _| {
                viewer_c.doing_button_motion.set(false);
                Inhibit(false)
            });
        let viewer_c = Rc::clone(&viewer);
        viewer
            .scrolled_window
            .connect_motion_notify_event(move |_, event| {
                if viewer_c.doing_button_motion.get() {
                    let this_xy: Point = event.get_position().into();
                    let delta_xy: [f64; 2] = (this_xy - viewer_c.last_xy.get()).into();
                    viewer_c.last_xy.set(this_xy);
                    for (dim, o_adj) in [
                        viewer_c.scrolled_window.get_hadjustment(),
                        viewer_c.scrolled_window.get_vadjustment(),
                    ]
                    .iter()
                    .enumerate()
                    {
                        if let Some(ref adj) = *o_adj {
                            let new_val = adj.get_value() - delta_xy[dim];
                            adj.set_value(
                                new_val
                                    .clamp(adj.get_lower(), adj.get_upper() - adj.get_page_size()),
                            );
                        }
                    }
                    return Inhibit(true);
                };
                Inhibit(false)
            });

        // POPUP MENU
        let viewer_c = Rc::clone(&viewer);
        viewer
            .popup_menu
            .append_item(
                "copy",
                &("Copy", None, Some("Copy the selection to the clipboard")).into(),
                PixbufView::SAV_HAS_IMAGE + PixbufView::SAV_HAS_SELECTION,
            )
            .expect("Duplicate menu item: copy")
            .connect_activate(move |_| {
                let zoomable = viewer_c.zoomable.borrow();
                let zoomable = zoomable.as_ref().expect("SAV_HAS_IMAGE");
                let rect = viewer_c
                    .xy_selection
                    .get_selected_rectangle()
                    .expect("SAV_HAS_SELECTION");
                let pixbuf = zoomable.subpixbuf(rect.into()).expect("programmer error");
                let cbd = gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD);
                cbd.set_image(&pixbuf);
            });

        let viewer_c = Rc::clone(&viewer);
        viewer
            .popup_menu
            .append_item(
                "load",
                &("Load", None, Some("Load an image from a nominated file.")).into(),
                0,
            )
            .expect("Duplicate menu item: load")
            .connect_activate(move |_| {
                let o_last_file = recollections::recall("image_viewer::last_image_file");
                if let Some(path) =
                    viewer_c.ask_file_path(Some("Image File"), o_last_file.as_deref(), true)
                {
                    if let Err(err) = viewer_c.set_pixbuf_fm_file(&path) {
                        viewer_c.report_error("Failed To Load Image", &err);
                    } else {
                        let o_path_str = path.to_str();
                        if let Some(path_str) = o_path_str {
                            recollections::remember("image_viewer::last_image_file", path_str);
                        }
                    }
                }
            });

        let viewer_c = Rc::clone(&viewer);
        viewer
            .popup_menu
            .append_item(
                "print",
                &("Print", None, Some("Print the image.")).into(),
                PixbufView::SAV_HAS_IMAGE,
            )
            .expect("Duplicate menu item: print")
            .connect_activate(move |_| {
                let zoomable = viewer_c.zoomable.borrow();
                let zoomable = zoomable.as_ref().expect("SAV_HAS_IMAGE");
                if let Err(ref err) = viewer_c.print_pixbuf(&zoomable.pixbuf()) {
                    viewer_c.report_error("Print Error", err);
                }
            });

        let viewer_c = Rc::clone(&viewer);
        viewer
            .popup_menu
            .append_item(
                "print selection",
                &(
                    "Print Selection",
                    None,
                    Some("Print the selectef part of the image"),
                )
                    .into(),
                PixbufView::SAV_HAS_IMAGE + PixbufView::SAV_HAS_SELECTION,
            )
            .expect("Duplicate menu item: print selection")
            .connect_activate(move |_| {
                let zoomable = viewer_c.zoomable.borrow();
                let zoomable = zoomable.as_ref().expect("SAV_HAS_IMAGE");
                let rect = viewer_c
                    .xy_selection
                    .get_selected_rectangle()
                    .expect("SAV_HAS_SELECTION");
                let pixbuf = zoomable.subpixbuf(rect.into()).expect("programmer error");
                if let Err(ref err) = viewer_c.print_pixbuf(&pixbuf) {
                    viewer_c.report_error("Print Error", err);
                }
            });

        if self.load_last_image {
            let o_last_file = recollections::recall("image_viewer::last_image_file");
            if let Some(ref last_file_path) = o_last_file {
                if let Err(err) = viewer.set_pixbuf_fm_file(last_file_path) {
                    viewer.report_error("Failed To Load Previous Image", &err);
                };
            };
        };

        viewer
    }
}

impl Default for PixbufViewBuilder {
    fn default() -> Self {
        Self::new()
    }
}
