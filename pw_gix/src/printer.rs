// Copyright 2017 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::cell::{Cell, RefCell};
use std::error::Error;
use std::fmt;
use std::fs;
use std::path;
use std::rc::Rc;
use std::result;

use mut_static::*;

use gdk::prelude::GdkContextExt;
use gdk_pixbuf;
use glib;
use gtk;
use gtk::prelude::{IsA, PrintOperationExt};
use pango;
use pangocairo;

struct RememberedPrinterSettings {
    pub o_file_path: Option<path::PathBuf>,
}

impl RememberedPrinterSettings {
    fn set_file_path(&mut self, file_path: &path::Path) {
        if !file_path.exists() {
            if let Some(dir_path) = file_path.parent() {
                if !dir_path.exists() {
                    fs::create_dir_all(dir_path).unwrap_or_else(|err| {
                        panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
                    });
                }
            };
            if let Err(err) = gtk::PrintSettings::new().to_file(file_path) {
                panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
            };
        };
        self.o_file_path = Some(file_path.to_path_buf())
    }
}

lazy_static! {
    static ref REMEMBERED_PRINTER_SETTINGS: MutStatic<RememberedPrinterSettings> =
        MutStatic::from(RememberedPrinterSettings { o_file_path: None });
}

pub fn init_printer(file_path: &path::Path) {
    REMEMBERED_PRINTER_SETTINGS
        .write()
        .unwrap()
        .set_file_path(file_path);
}

fn get_printer_settings() -> gtk::PrintSettings {
    let settings = gtk::PrintSettings::new();
    if let Some(ref file_path) = REMEMBERED_PRINTER_SETTINGS.write().unwrap().o_file_path {
        if let Err(err) = settings.load_file(file_path) {
            panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
        }
    };
    settings
}

fn save_printer_settings(settings: &gtk::PrintSettings) {
    if let Some(ref file_path) = REMEMBERED_PRINTER_SETTINGS.write().unwrap().o_file_path {
        if let Err(err) = settings.to_file(file_path) {
            panic!("{:?}: line {:?}: {:?}", file!(), line!(), err)
        }
    };
}

#[derive(Debug)]
pub struct PrintError(Option<glib::Error>);

impl fmt::Display for PrintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PrintError({:?}): {:?}.", self.0, self.source())?;
        Ok(())
    }
}

impl Error for PrintError {
    fn description(&self) -> &str {
        "Printing failed."
    }

    fn cause(&self) -> Option<&dyn Error> {
        if let Some(ref error) = self.0 {
            Some(error)
        } else {
            None
        }
    }
}

impl From<glib::Error> for PrintError {
    fn from(glib_error: glib::Error) -> PrintError {
        PrintError(Some(glib_error))
    }
}

pub type PrintResult = result::Result<(), PrintError>;

fn do_print_operation<P: IsA<gtk::Window>>(
    print_operation: &gtk::PrintOperation,
    parent: Option<&P>,
) -> PrintResult {
    let result = print_operation.run(gtk::PrintOperationAction::PrintDialog, parent)?;
    if result == gtk::PrintOperationResult::Error {
        return Err(PrintError(None));
    } else if result == gtk::PrintOperationResult::Apply {
        if let Some(settings) = print_operation.get_print_settings() {
            save_printer_settings(&settings);
        }
    };
    Ok(())
}

struct MarkupPrinterCore {
    print_operation: gtk::PrintOperation,
    chunks: Vec<String>,
    pages: RefCell<Vec<Vec<pango::Layout>>>,
}

trait MarkupPrinterInterface {
    fn create(chunks: Vec<String>) -> Rc<MarkupPrinterCore>;
}

impl MarkupPrinterInterface for Rc<MarkupPrinterCore> {
    // NB: this is all necessary because of need for callbacks
    fn create(chunks: Vec<String>) -> Rc<MarkupPrinterCore> {
        let mp = Rc::new(MarkupPrinterCore {
            print_operation: gtk::PrintOperation::new(),
            chunks,
            pages: RefCell::new(Vec::new()),
        });
        mp.print_operation
            .set_print_settings(Some(&get_printer_settings()));
        mp.print_operation.set_unit(gtk::Unit::Mm);

        let mp_c = mp.clone();
        mp.print_operation
            .connect_begin_print(move |pr_op, pr_ctxt| {
                let pheight = pr_ctxt.get_height() as i32;
                let spwidth = (pr_ctxt.get_width() * pango::SCALE as f64) as i32;
                let mut total_height: i32 = 0;
                let mut page: Vec<pango::Layout> = Vec::new();
                for chunk in mp_c.chunks.iter() {
                    if let Some(layout) = pr_ctxt.create_pango_layout() {
                        layout.set_width(spwidth);
                        layout.set_markup(chunk);
                        let (_, c_height) = layout.get_pixel_size();
                        if (total_height + c_height) < pheight {
                            page.push(layout);
                            total_height += c_height;
                        } else {
                            mp_c.pages.borrow_mut().push(page);
                            page = vec![layout];
                            total_height = c_height;
                            // TODO: handle case where markup too big for one page
                        }
                    } else {
                        panic!("File: {} Line: {}", file!(), line!());
                    }
                }
                if !page.is_empty() {
                    mp_c.pages.borrow_mut().push(page);
                };
                pr_op.set_n_pages(mp_c.pages.borrow().len() as i32);
            });

        let mp_c = mp.clone();
        mp.print_operation
            .connect_draw_page(move |_, pr_ctxt, page_num| {
                let layouts = &mp_c.pages.borrow()[page_num as usize];
                let mut y: f64 = 0.0;
                for layout in layouts.iter() {
                    if let Some(cairo_context) = pr_ctxt.get_cairo_context() {
                        cairo_context.move_to(0.0, y);
                        pangocairo::functions::show_layout(&cairo_context, layout);
                        let (_, h) = layout.get_pixel_size();
                        y += h as f64;
                    } else {
                        panic!("File: {} Line: {}", file!(), line!());
                    }
                }
            });

        mp
    }
}

type MarkupPrinter = Rc<MarkupPrinterCore>;

pub fn print_markup_chunks<P: IsA<gtk::Window>>(
    chunks: Vec<String>,
    parent: Option<&P>,
) -> PrintResult {
    let markup_printer = MarkupPrinter::create(chunks);
    do_print_operation(&markup_printer.print_operation, parent)
}

struct PixbufPrinterCore {
    print_operation: gtk::PrintOperation,
    pixbuf: RefCell<gdk_pixbuf::Pixbuf>,
}

trait PixbufPrinterInterface {
    fn create(pixbuf: &gdk_pixbuf::Pixbuf) -> Rc<PixbufPrinterCore>;
}

impl PixbufPrinterInterface for Rc<PixbufPrinterCore> {
    // NB: this is all necessary because of need for callbacks
    fn create(pixbuf: &gdk_pixbuf::Pixbuf) -> Rc<PixbufPrinterCore> {
        let mp = Rc::new(PixbufPrinterCore {
            print_operation: gtk::PrintOperation::new(),
            pixbuf: RefCell::new(pixbuf.clone()),
        });
        mp.print_operation
            .set_print_settings(Some(&get_printer_settings()));
        mp.print_operation.set_unit(gtk::Unit::Mm);

        let mp_c = mp.clone();
        mp.print_operation
            .connect_begin_print(move |pr_op, pr_ctxt| {
                let pixbuf = mp_c.pixbuf.borrow().clone();
                let pheight = pixbuf.get_height();
                let pwidth = pixbuf.get_width();
                // TODO: use this code when pixbuf rotation available
                //let mut pixbuf = mp_c.pixbuf.borrow().clone();
                //let mut pheight = pixbuf.get_height();
                //let mut pwidth = pixbuf.get_width();
                //if pwidth > pheight {
                //pixbuf = pixbuf.rotate_simple(gdk_pixbuf::PixbufRotation::Clockwise);
                //pheight = pixbuf.get_height();
                //pwidth = pixbuf.get_width();
                //};
                let hscale = pr_ctxt.get_height() / pheight as f64;
                let wscale = pr_ctxt.get_width() / pwidth as f64;
                let scale = hscale.min(wscale);
                let new_width = (pwidth as f64 * scale).round() as i32;
                let new_height = (pheight as f64 * scale).round() as i32;
                if let Some(new_pixbuf) =
                    pixbuf.scale_simple(new_width, new_height, gdk_pixbuf::InterpType::Bilinear)
                {
                    *mp_c.pixbuf.borrow_mut() = new_pixbuf
                } else {
                    panic!("File: {} Line: {}", file!(), line!())
                };
                pr_op.set_n_pages(1);
            });

        let mp_c = mp.clone();
        mp.print_operation.connect_draw_page(move |_, pr_ctxt, _| {
            let pixbuf = mp_c.pixbuf.borrow();
            if let Some(cairo_context) = pr_ctxt.get_cairo_context() {
                cairo_context.set_source_pixbuf(&pixbuf, 0.0, 0.0);
                cairo_context.paint();
            } else {
                panic!("File: {} Line: {}", file!(), line!());
            }
        });

        mp
    }
}

type PixbufPrinter = Rc<PixbufPrinterCore>;

pub fn print_pixbuf<P: IsA<gtk::Window>>(
    pixbuf: &gdk_pixbuf::Pixbuf,
    parent: Option<&P>,
) -> PrintResult {
    let pixbuf_printer = PixbufPrinter::create(pixbuf);
    do_print_operation(&pixbuf_printer.print_operation, parent)
}

struct TextPrinterCore {
    print_operation: gtk::PrintOperation,
    layout: RefCell<Option<pango::Layout>>,
    next_line_index: Cell<i32>,
}

trait TextPrinterInterface {
    fn create(text: &str) -> Rc<TextPrinterCore>;
}

impl TextPrinterInterface for Rc<TextPrinterCore> {
    // NB: this is all necessary because of need for callbacks
    fn create(text: &str) -> Rc<TextPrinterCore> {
        let mp = Rc::new(TextPrinterCore {
            print_operation: gtk::PrintOperation::new(),
            layout: RefCell::new(None),
            next_line_index: Cell::new(0),
        });
        mp.print_operation
            .set_print_settings(Some(&get_printer_settings()));
        mp.print_operation.set_unit(gtk::Unit::Mm);

        let mp_c = mp.clone();
        let text_c = text.to_string();
        mp.print_operation
            .connect_begin_print(move |pr_op, pr_ctxt| {
                if let Some(layout) = pr_ctxt.create_pango_layout() {
                    let spwidth = (pr_ctxt.get_width() * pango::SCALE as f64) as i32;
                    layout.set_width(spwidth);
                    layout.set_text(&text_c);
                    let (_, t_height) = layout.get_pixel_size();
                    let l_height = t_height as f64 / layout.get_line_count() as f64;
                    let lpp = (pr_ctxt.get_height() / l_height).floor();
                    let np = layout.get_line_count() as f64 / lpp;
                    pr_op.set_n_pages((np + 1.0) as i32);
                    *mp_c.layout.borrow_mut() = Some(layout);
                } else {
                    panic!("File: {} Line: {}", file!(), line!());
                }
            });

        let mp_c = mp.clone();
        mp.print_operation.connect_draw_page(move |_, pr_ctxt, _| {
            if let Some(ref layout) = *mp_c.layout.borrow_mut() {
                if let Some(cairo_context) = pr_ctxt.get_cairo_context() {
                    let page_height = pr_ctxt.get_height();
                    let mut y: f64 = 0.0;
                    let mut index = mp_c.next_line_index.get();
                    while y < page_height {
                        cairo_context.move_to(0.0, y);
                        if let Some(layout_line) = layout.get_line(index) {
                            cairo_context.move_to(0.0, y);
                            pangocairo::functions::show_layout_line(&cairo_context, &layout_line);
                            let (_, logical_extent) = layout_line.get_pixel_extents();
                            y += logical_extent.height as f64;
                            index += 1;
                        } else {
                            break;
                        }
                    }
                    mp_c.next_line_index.set(index);
                } else {
                    panic!("File: {} Line: {}", file!(), line!());
                }
            } else {
                panic!("File: {:?} Line: {:?}", file!(), line!())
            };
        });

        mp
    }
}

type TextPrinter = Rc<TextPrinterCore>;

pub fn print_text<P: IsA<gtk::Window>>(text: &str, parent: Option<&P>) -> PrintResult {
    let text_printer = TextPrinter::create(text);
    do_print_operation(&text_printer.print_operation, parent)
}
